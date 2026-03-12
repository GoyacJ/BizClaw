use std::{env, process::Command};

use anyhow::{anyhow, Context, Result};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::{
    config_store::JsonSettingsStore,
    install::{
        command_available, current_platform, elevated_install_plan, fallback_install_plans,
        looks_like_permission_error, official_install_plan, InstallPlan, MANUAL_INSTALL_URL,
    },
    runtime_supervisor::{
        mark_configured, new_shared_runtime_state, snapshot_logs, snapshot_status,
        start_runtime_processes, stop_runtime_processes, SharedRuntimeState,
    },
    secret_store::{KeyringSecretStore, SecretStore},
    types::{
        CompanyProfile, EnvironmentSnapshot, InstallRequest, InstallResult, LogEntry,
        PersistedSettings, RuntimePhase, RuntimeStatus, UserProfile,
    },
    validation::{
        saved_settings_are_complete, validate_company_profile, validate_user_profile,
    },
};

pub struct AppState {
    pub runtime: SharedRuntimeState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            runtime: new_shared_runtime_state(),
        }
    }
}

#[tauri::command]
pub fn detect_environment(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EnvironmentSnapshot, String> {
    let settings = settings_store(&app)
        .and_then(|store| store.load())
        .map_err(err_to_string)?;

    let openclaw_installed = command_available("openclaw");
    let has_saved_profile = settings
        .as_ref()
        .map(saved_settings_are_complete)
        .unwrap_or(false);
    let has_saved_token = KeyringSecretStore::new()
        .and_then(|store| store.get_token())
        .map(|token| token.is_some())
        .unwrap_or(false);
    let mut runtime_status = snapshot_status(&state.runtime);
    if !openclaw_installed {
        runtime_status.phase = RuntimePhase::InstallNeeded;
    } else if has_saved_profile && matches!(runtime_status.phase, RuntimePhase::Checking) {
        runtime_status.phase = RuntimePhase::Configured;
    }

    Ok(EnvironmentSnapshot {
        os: env::consts::OS.to_string(),
        ssh_installed: command_available("ssh"),
        openclaw_installed,
        npm_installed: command_available("npm"),
        pnpm_installed: command_available("pnpm"),
        has_saved_profile,
        has_saved_token,
        saved_settings: settings,
        runtime_status,
        install_recommendation: install_recommendation(),
    })
}

#[tauri::command]
pub fn install_openclaw(
    _app: AppHandle,
    request: InstallRequest,
) -> Result<InstallResult, String> {
    if command_available("openclaw") {
        return Ok(InstallResult {
            strategy: "skipped".into(),
            success: true,
            stdout: String::new(),
            stderr: String::new(),
            needs_elevation: false,
            manual_url: MANUAL_INSTALL_URL.into(),
            follow_up: "已检测到 openclaw，跳过安装。".into(),
        });
    }

    let platform = current_platform().map_err(err_to_string)?;
    let mut plans = Vec::new();

    if request.prefer_official {
        plans.push(official_install_plan(platform));
    }
    plans.extend(fallback_install_plans(
        command_available("npm"),
        command_available("pnpm"),
    ));

    if plans.is_empty() {
        return Err("当前系统缺少可用安装方式，请改用手动安装。".into());
    }

    let mut last_result = None;
    for plan in plans {
        let output = execute_plan(&plan).map_err(err_to_string)?;
        let mut result = build_install_result(&plan, output);
        if result.success || command_available("openclaw") {
            result.success = true;
            result.follow_up = "安装命令执行完成，请返回界面重新检测。".into();
            return Ok(result);
        }

        if result.needs_elevation && request.allow_elevation {
            if let Some(elevated_plan) = elevated_install_plan(&plan, platform) {
                let elevated_output = execute_plan(&elevated_plan).map_err(err_to_string)?;
                let mut elevated_result = build_install_result(&elevated_plan, elevated_output);
                if elevated_result.success || command_available("openclaw") {
                    elevated_result.success = true;
                    elevated_result.follow_up =
                        "已尝试管理员权限安装，请返回界面重新检测。".into();
                    return Ok(elevated_result);
                }
                last_result = Some(elevated_result);
                continue;
            }
        }

        last_result = Some(result);
    }

    Ok(last_result.unwrap_or_else(|| InstallResult {
        strategy: "unknown".into(),
        success: false,
        stdout: String::new(),
        stderr: "安装未执行。".into(),
        needs_elevation: false,
        manual_url: MANUAL_INSTALL_URL.into(),
        follow_up: "请改用手动安装。".into(),
    }))
}

#[tauri::command]
pub fn open_manual_install(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(MANUAL_INSTALL_URL, None::<&str>)
        .map_err(err_to_string)
}

#[tauri::command]
pub fn save_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    company_profile: CompanyProfile,
    user_profile: UserProfile,
    token: String,
) -> Result<PersistedSettings, String> {
    validate_company_profile(&company_profile).map_err(err_to_string)?;
    validate_user_profile(&user_profile).map_err(err_to_string)?;
    let normalized_token = if token.trim().is_empty() {
        KeyringSecretStore::new()
            .and_then(|store| {
                store
                    .get_token()?
                    .ok_or_else(|| anyhow!("Token 不能为空"))
            })
            .map_err(err_to_string)?
    } else {
        token.trim().to_string()
    };

    let settings = PersistedSettings {
        company_profile,
        user_profile,
    };
    settings_store(&app)
        .and_then(|store| store.save(&settings))
        .map_err(err_to_string)?;

    KeyringSecretStore::new()
        .and_then(|store| store.set_token(&normalized_token))
        .map_err(err_to_string)?;

    mark_configured(&state.runtime, &app);
    Ok(settings)
}

#[tauri::command]
pub fn start_runtime(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let settings = settings_store(&app)
        .and_then(|store| {
            store
                .load()?
                .ok_or_else(|| anyhow!("尚未保存 BizClaw 连接配置"))
        })
        .map_err(err_to_string)?;
    validate_company_profile(&settings.company_profile).map_err(err_to_string)?;
    validate_user_profile(&settings.user_profile).map_err(err_to_string)?;

    let token = KeyringSecretStore::new()
        .and_then(|store| {
            store
                .get_token()?
                .ok_or_else(|| anyhow!("尚未保存 OPENCLAW_GATEWAY_TOKEN"))
        })
        .map_err(err_to_string)?;

    start_runtime_processes(
        state.runtime.clone(),
        app,
        &settings.company_profile,
        &settings.user_profile,
        &token,
    )
    .map_err(err_to_string)
}

#[tauri::command]
pub fn stop_runtime(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    stop_runtime_processes(state.runtime.clone(), app).map_err(err_to_string)
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    Ok(snapshot_status(&state.runtime))
}

#[tauri::command]
pub fn stream_logs(state: State<'_, AppState>) -> Result<Vec<LogEntry>, String> {
    Ok(snapshot_logs(&state.runtime))
}

fn settings_store(app: &AppHandle) -> Result<JsonSettingsStore> {
    let path = app
        .path()
        .app_data_dir()
        .context("无法获取应用数据目录")?
        .join("settings.json");
    Ok(JsonSettingsStore::new(path))
}

fn install_recommendation() -> String {
    if env::consts::OS == "windows" {
        "Windows 环境建议优先确认 OpenSSH 可用；如遇兼容问题，OpenClaw 官方更推荐 WSL2。"
            .into()
    } else {
        "推荐先使用官方安装脚本，失败后再回退到 npm / pnpm 全局安装。".into()
    }
}

fn execute_plan(plan: &InstallPlan) -> Result<std::process::Output> {
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    command
        .output()
        .with_context(|| format!("执行安装命令失败: {}", plan.program))
}

fn build_install_result(plan: &InstallPlan, output: std::process::Output) -> InstallResult {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let merged = format!("{stdout}\n{stderr}");

    InstallResult {
        strategy: plan.strategy.into(),
        success: output.status.success(),
        stdout,
        stderr,
        needs_elevation: looks_like_permission_error(&merged),
        manual_url: MANUAL_INSTALL_URL.into(),
        follow_up: "安装失败后可改用手动安装或管理员权限重试。".into(),
    }
}

fn err_to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
