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
    secret_store::{LocalSecretStore, SecretStore},
    types::{
        CompanyProfile, EnvironmentSnapshot, InstallRequest, InstallResult, LogEntry,
        PersistedSettings, RuntimePhase, RuntimeStatus, TokenStatus, UserProfile,
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

trait SettingsStoreAccess {
    fn load(&self) -> Result<Option<PersistedSettings>>;
    fn save(&self, settings: &PersistedSettings) -> Result<()>;
}

impl SettingsStoreAccess for JsonSettingsStore {
    fn load(&self) -> Result<Option<PersistedSettings>> {
        JsonSettingsStore::load(self)
    }

    fn save(&self, settings: &PersistedSettings) -> Result<()> {
        JsonSettingsStore::save(self, settings)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenState {
    status: TokenStatus,
    message: Option<String>,
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
    let openclaw_version = openclaw_installed.then(read_openclaw_version).flatten();
    let token_state = inspect_token_state(token_store(&app).and_then(|store| store.get_secret()));

    Ok(build_environment_snapshot(
        env::consts::OS,
        settings,
        snapshot_status(&state.runtime),
        command_available("ssh"),
        openclaw_installed,
        openclaw_version,
        command_available("npm"),
        command_available("pnpm"),
        token_state,
    ))
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
    ssh_password: String,
) -> Result<PersistedSettings, String> {
    let settings = PersistedSettings {
        company_profile,
        user_profile,
    };
    let store = settings_store(&app).map_err(err_to_string)?;
    let token_store = token_store(&app).map_err(err_to_string)?;
    let ssh_password_store = ssh_password_store(&app).map_err(err_to_string)?;
    persist_profile_atomic(
        &store,
        &token_store,
        &ssh_password_store,
        &settings,
        &token,
        &ssh_password,
    )
    .map_err(err_to_string)?;

    mark_configured(&state.runtime, &app);
    Ok(settings)
}

#[tauri::command]
pub fn start_runtime(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let settings_store = settings_store(&app).map_err(err_to_string)?;
    let settings = load_saved_settings(&settings_store).map_err(err_to_string)?;
    validate_company_profile(&settings.company_profile).map_err(err_to_string)?;
    validate_user_profile(&settings.user_profile).map_err(err_to_string)?;

    let secret_store = token_store(&app).map_err(err_to_string)?;
    let token = load_saved_token(&secret_store).map_err(err_to_string)?;
    let ssh_password = ssh_password_store(&app)
        .and_then(|store| store.get_secret())
        .map_err(err_to_string)?;

    start_runtime_processes(
        state.runtime.clone(),
        app,
        &settings.company_profile,
        &settings.user_profile,
        &token,
        ssh_password.as_deref(),
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

fn token_store(app: &AppHandle) -> Result<LocalSecretStore> {
    let path = app
        .path()
        .app_data_dir()
        .context("无法获取应用数据目录")?
        .join("gateway-token.txt");
    Ok(LocalSecretStore::new(path))
}

fn ssh_password_store(app: &AppHandle) -> Result<LocalSecretStore> {
    let path = app
        .path()
        .app_data_dir()
        .context("无法获取应用数据目录")?
        .join("ssh-password.txt");
    Ok(LocalSecretStore::new(path))
}

fn build_environment_snapshot(
    os: &str,
    settings: Option<PersistedSettings>,
    mut runtime_status: RuntimeStatus,
    ssh_installed: bool,
    openclaw_installed: bool,
    openclaw_version: Option<String>,
    npm_installed: bool,
    pnpm_installed: bool,
    token_state: TokenState,
) -> EnvironmentSnapshot {
    let has_saved_profile = settings
        .as_ref()
        .map(saved_settings_are_complete)
        .unwrap_or(false);

    if !openclaw_installed {
        runtime_status.phase = RuntimePhase::InstallNeeded;
    } else if has_saved_profile && matches!(runtime_status.phase, RuntimePhase::Checking) {
        runtime_status.phase = RuntimePhase::Configured;
    }

    EnvironmentSnapshot {
        os: os.to_string(),
        ssh_installed,
        openclaw_installed,
        openclaw_version,
        npm_installed,
        pnpm_installed,
        has_saved_profile,
        token_status: token_state.status,
        token_status_message: token_state.message,
        saved_settings: settings,
        runtime_status,
        install_recommendation: install_recommendation(),
    }
}

fn inspect_token_state(result: Result<Option<String>>) -> TokenState {
    match result {
        Ok(Some(_)) => TokenState {
            status: TokenStatus::Saved,
            message: None,
        },
        Ok(None) => TokenState {
            status: TokenStatus::Missing,
            message: None,
        },
        Err(error) => TokenState {
            status: TokenStatus::Error,
            message: Some(error.to_string()),
        },
    }
}

fn read_openclaw_version() -> Option<String> {
    let output = Command::new("openclaw").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return Some(stdout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    (!stderr.is_empty()).then_some(stderr)
}

fn persist_profile_atomic(
    settings_store: &impl SettingsStoreAccess,
    token_store: &impl SecretStore,
    ssh_password_store: &impl SecretStore,
    settings: &PersistedSettings,
    token: &str,
    ssh_password: &str,
) -> Result<()> {
    validate_company_profile(&settings.company_profile)?;
    validate_user_profile(&settings.user_profile)?;

    let previous_token = token_store.get_secret()?;
    let previous_ssh_password = ssh_password_store.get_secret()?;
    let trimmed_token = token.trim();
    let trimmed_ssh_password = ssh_password.trim();
    let should_write_new_token = !trimmed_token.is_empty();
    let should_write_new_ssh_password = !trimmed_ssh_password.is_empty();

    if should_write_new_token {
        token_store.set_secret(trimmed_token)?;
    } else if previous_token.is_none() {
        return Err(anyhow!("Token 不能为空"));
    }

    if should_write_new_ssh_password {
        if let Err(save_error) = ssh_password_store.set_secret(trimmed_ssh_password) {
            if should_write_new_token {
                rollback_secret(token_store, previous_token.as_deref()).map_err(
                    |rollback_error| anyhow!("{save_error}; token 回滚失败: {rollback_error}"),
                )?;
            }
            return Err(save_error);
        }
    }

    if let Err(save_error) = settings_store.save(settings) {
        if should_write_new_token {
            rollback_secret(token_store, previous_token.as_deref())
                .map_err(|rollback_error| anyhow!("{save_error}; token 回滚失败: {rollback_error}"))?;
        }
        if should_write_new_ssh_password {
            rollback_secret(ssh_password_store, previous_ssh_password.as_deref()).map_err(
                |rollback_error| anyhow!("{save_error}; SSH 密码回滚失败: {rollback_error}"),
            )?;
        }
        return Err(save_error);
    }

    Ok(())
}

fn rollback_secret(secret_store: &impl SecretStore, previous_secret: Option<&str>) -> Result<()> {
    if let Some(value) = previous_secret {
        secret_store.set_secret(value)
    } else {
        secret_store.clear_secret()
    }
}

fn load_saved_settings(settings_store: &impl SettingsStoreAccess) -> Result<PersistedSettings> {
    settings_store
        .load()?
        .ok_or_else(|| anyhow!("尚未保存 BizClaw 连接配置"))
}

fn load_saved_token(secret_store: &impl SecretStore) -> Result<String> {
    secret_store
        .get_secret()?
        .ok_or_else(|| anyhow!("尚未保存 OPENCLAW_GATEWAY_TOKEN"))
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use anyhow::{anyhow, Result};

    use super::{
        build_environment_snapshot, inspect_token_state, load_saved_token,
        persist_profile_atomic, RuntimePhase, RuntimeStatus, SettingsStoreAccess, TokenState,
    };
    use crate::{
        secret_store::{MemorySecretStore, SecretStore},
        types::{CompanyProfile, PersistedSettings, TokenStatus, UserProfile},
    };

    #[derive(Clone, Default)]
    struct FakeSettingsStore {
        loaded: Option<PersistedSettings>,
        saved: Arc<Mutex<Option<PersistedSettings>>>,
        save_error: Option<String>,
    }

    impl SettingsStoreAccess for FakeSettingsStore {
        fn load(&self) -> Result<Option<PersistedSettings>> {
            Ok(self.loaded.clone())
        }

        fn save(&self, settings: &PersistedSettings) -> Result<()> {
            if let Some(message) = &self.save_error {
                return Err(anyhow!(message.clone()));
            }

            *self.saved.lock().expect("saved settings mutex poisoned") = Some(settings.clone());
            Ok(())
        }
    }

    struct FailingSecretStore;

    impl SecretStore for FailingSecretStore {
        fn set_secret(&self, _value: &str) -> Result<()> {
            Err(anyhow!("token store unavailable"))
        }

        fn get_secret(&self) -> Result<Option<String>> {
            Ok(None)
        }

        fn clear_secret(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn environment_snapshot_keeps_openclaw_version_when_installed() {
        let snapshot = build_environment_snapshot(
            "macos",
            Some(sample_settings()),
            RuntimeStatus::default(),
            true,
            true,
            Some("OpenClaw 2026.3.8".into()),
            true,
            true,
            TokenState {
                status: TokenStatus::Saved,
                message: None,
            },
        );

        assert_eq!(snapshot.openclaw_version.as_deref(), Some("OpenClaw 2026.3.8"));
        assert_eq!(snapshot.token_status, TokenStatus::Saved);
        assert_eq!(snapshot.runtime_status.phase, RuntimePhase::Configured);
    }

    #[test]
    fn inspect_token_state_reports_missing_token() {
        let token_state = inspect_token_state(Ok(None));

        assert_eq!(token_state.status, TokenStatus::Missing);
        assert_eq!(token_state.message, None);
    }

    #[test]
    fn inspect_token_state_reports_storage_errors() {
        let token_state = inspect_token_state(Err(anyhow!("token file unavailable")));

        assert_eq!(token_state.status, TokenStatus::Error);
        assert_eq!(token_state.message.as_deref(), Some("token file unavailable"));
    }

    #[test]
    fn atomic_save_stops_before_writing_settings_when_token_write_fails() {
        let settings_store = FakeSettingsStore::default();
        let error = persist_profile_atomic(
            &settings_store,
            &FailingSecretStore,
            &MemorySecretStore::default(),
            &sample_settings(),
            "gateway-token",
            "",
        )
        .unwrap_err();

        assert!(error.to_string().contains("token store unavailable"));
        assert_eq!(
            settings_store
                .saved
                .lock()
                .expect("saved settings mutex poisoned")
                .clone(),
            None,
        );
    }

    #[test]
    fn atomic_save_rolls_back_new_token_when_settings_save_fails() {
        let token_store = MemorySecretStore::default();
        token_store.set_secret("old-token").unwrap();
        let ssh_password_store = MemorySecretStore::default();
        ssh_password_store.set_secret("old-password").unwrap();
        let settings_store = FakeSettingsStore {
            save_error: Some("settings save failed".into()),
            ..FakeSettingsStore::default()
        };

        let error = persist_profile_atomic(
            &settings_store,
            &token_store,
            &ssh_password_store,
            &sample_settings(),
            "new-token",
            "new-password",
        )
        .unwrap_err();

        assert!(error.to_string().contains("settings save failed"));
        assert_eq!(token_store.get_secret().unwrap().as_deref(), Some("old-token"));
        assert_eq!(
            ssh_password_store.get_secret().unwrap().as_deref(),
            Some("old-password"),
        );
    }

    #[test]
    fn atomic_save_keeps_existing_ssh_password_when_input_is_blank() {
        let token_store = MemorySecretStore::default();
        token_store.set_secret("gateway-token").unwrap();
        let ssh_password_store = MemorySecretStore::default();
        ssh_password_store.set_secret("saved-password").unwrap();

        persist_profile_atomic(
            &FakeSettingsStore::default(),
            &token_store,
            &ssh_password_store,
            &sample_settings(),
            "",
            "",
        )
        .unwrap();

        assert_eq!(
            ssh_password_store.get_secret().unwrap().as_deref(),
            Some("saved-password"),
        );
    }

    #[test]
    fn load_saved_token_requires_existing_secret() {
        let error = load_saved_token(&MemorySecretStore::default()).unwrap_err();

        assert!(error.to_string().contains("OPENCLAW_GATEWAY_TOKEN"));
    }

    fn sample_settings() -> PersistedSettings {
        PersistedSettings {
            company_profile: CompanyProfile {
                ssh_host: "127.0.0.1".into(),
                ssh_user: "root".into(),
                local_port: 18889,
                remote_bind_host: "61.150.94.14".into(),
                remote_bind_port: 18789,
            },
            user_profile: UserProfile {
                display_name: "Goya Mac".into(),
                auto_connect: true,
                run_in_background: true,
            },
        }
    }
}
