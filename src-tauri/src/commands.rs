use std::{
    env,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::mpsc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::{
    app_menu,
    config_store::JsonSettingsStore,
    install::{
        command_available, compare_versions, current_platform, default_runtime_target,
        detect_wsl_status, elevated_install_plan, fallback_install_plans_for_target,
        looks_like_permission_error, official_install_plan_for_target,
        read_latest_openclaw_version, read_openclaw_version, target_command_available,
        wsl_bootstrap_plan, wsl_ensure_ssh_plan, InstallPlan, Platform, MANUAL_INSTALL_URL,
    },
    operation_supervisor::{
        attach_child, cancel_requested, clear_child, finish_task, new_shared_operation_state,
        push_event, request_stop, snapshot_events, snapshot_task, start_task, update_step,
        with_child_mut, SharedOperationState,
    },
    runtime::{
        build_native_gateway_status_command, build_native_ssh_command,
        build_wsl_gateway_status_command, build_wsl_ssh_command, CommandSpec,
    },
    runtime_supervisor::{
        mark_configured, new_shared_runtime_state, snapshot_logs, snapshot_status,
        start_runtime_processes, stop_runtime_processes, SharedRuntimeState,
    },
    secret_store::{LocalSecretStore, SecretStore},
    types::{
        CompanyProfile, ConnectionTestEvent, ConnectionTestEventStatus, ConnectionTestResult,
        ConnectionTestStep, EnvironmentSnapshot, InstallRequest, LogEntry, OperationEvent,
        OperationEventSource, OperationEventStatus, OperationKind, OperationResult, OperationStep,
        OperationTaskPhase, OperationTaskSnapshot, PersistedSettings, RuntimePhase, RuntimeStatus,
        RuntimeTarget, TargetProfile, TokenStatus, UserProfile, WslStatus,
    },
    validation::{
        saved_settings_are_complete, validate_company_profile, validate_target_profile,
        validate_user_profile,
    },
};

pub struct AppState {
    pub runtime: SharedRuntimeState,
    pub operation: SharedOperationState,
    pub port_task_active: Arc<AtomicBool>,
    pub environment_cache: Arc<Mutex<Option<EnvironmentSnapshot>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            runtime: new_shared_runtime_state(),
            operation: new_shared_operation_state(),
            port_task_active: Arc::new(AtomicBool::new(false)),
            environment_cache: Arc::new(Mutex::new(None)),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedEnvironment {
    runtime_target: RuntimeTarget,
    host_ssh_installed: bool,
    target_ssh_installed: bool,
    openclaw_installed: bool,
    openclaw_version: Option<String>,
    latest_openclaw_version: Option<String>,
    update_available: bool,
    wsl_status: Option<WslStatus>,
}

#[derive(Debug)]
struct StreamedPlanResult {
    strategy: String,
    success: bool,
    stdout: String,
    stderr: String,
    needs_elevation: bool,
}

struct PortTaskGuard {
    flag: Arc<AtomicBool>,
}

impl PortTaskGuard {
    fn acquire(flag: Arc<AtomicBool>) -> Result<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| anyhow!("当前有连接操作进行中，请稍候"))?;
        Ok(Self { flag })
    }
}

impl Drop for PortTaskGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
    }
}

const CONNECTION_TEST_TIMEOUT_MS: u64 = 8_000;
const CONNECTION_TEST_TUNNEL_READY_TIMEOUT: Duration = Duration::from_secs(6);
const CONNECTION_TEST_TUNNEL_POLL_INTERVAL: Duration = Duration::from_millis(150);

#[tauri::command]
pub fn detect_environment(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EnvironmentSnapshot, String> {
    refresh_environment_snapshot(&app, &state).map_err(err_to_string)
}

#[tauri::command]
pub fn install_openclaw(
    app: AppHandle,
    state: State<'_, AppState>,
    request: InstallRequest,
) -> Result<OperationTaskSnapshot, String> {
    let snapshot = refresh_environment_snapshot(&app, &state).map_err(err_to_string)?;
    start_install_or_update_task(
        &app,
        &state,
        OperationKind::Install,
        request,
        snapshot.runtime_target,
        snapshot
            .saved_settings
            .as_ref()
            .map(|settings| settings.target_profile.clone())
            .unwrap_or_default(),
    )
    .map_err(err_to_string)
}

#[tauri::command]
pub fn check_openclaw_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EnvironmentSnapshot, String> {
    emit_operation_event(
        &app,
        Some(&state.operation),
        OperationKind::CheckUpdate,
        OperationStep::CheckUpdate,
        OperationEventStatus::Running,
        OperationEventSource::System,
        "正在检查 OpenClaw 更新",
    );

    let snapshot = refresh_environment_snapshot(&app, &state).map_err(err_to_string)?;
    let message = if snapshot.update_available {
        format!(
            "检测到新版本：{}",
            snapshot
                .latest_openclaw_version
                .clone()
                .unwrap_or_else(|| "未知版本".into())
        )
    } else {
        "当前已是最新版本，或暂时无法获取远端版本信息。".into()
    };

    emit_operation_event(
        &app,
        Some(&state.operation),
        OperationKind::CheckUpdate,
        OperationStep::CheckUpdate,
        OperationEventStatus::Success,
        OperationEventSource::System,
        message,
    );

    Ok(snapshot)
}

#[tauri::command]
pub fn update_openclaw(
    app: AppHandle,
    state: State<'_, AppState>,
    request: InstallRequest,
) -> Result<OperationTaskSnapshot, String> {
    let snapshot = refresh_environment_snapshot(&app, &state).map_err(err_to_string)?;
    start_install_or_update_task(
        &app,
        &state,
        OperationKind::Update,
        request,
        snapshot.runtime_target,
        snapshot
            .saved_settings
            .as_ref()
            .map(|settings| settings.target_profile.clone())
            .unwrap_or_default(),
    )
    .map_err(err_to_string)
}

#[tauri::command]
pub fn get_operation_status(state: State<'_, AppState>) -> Result<OperationTaskSnapshot, String> {
    Ok(snapshot_task(&state.operation))
}

#[tauri::command]
pub fn get_operation_events(state: State<'_, AppState>) -> Result<Vec<OperationEvent>, String> {
    Ok(snapshot_events(&state.operation))
}

#[tauri::command]
pub fn stop_openclaw_operation(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OperationTaskSnapshot, String> {
    let snapshot = request_stop(&state.operation).map_err(err_to_string)?;
    emit_operation_status(&app, &snapshot);
    Ok(snapshot)
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
    target_profile: TargetProfile,
    token: String,
    ssh_password: String,
) -> Result<PersistedSettings, String> {
    let settings = PersistedSettings {
        company_profile,
        user_profile,
        target_profile,
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
    let _ = refresh_environment_snapshot(&app, &state);
    Ok(settings)
}

#[tauri::command]
pub async fn test_connection(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, String> {
    let guard = PortTaskGuard::acquire(state.port_task_active.clone()).map_err(err_to_string)?;
    let runtime_status = snapshot_status(&state.runtime);
    if matches!(
        runtime_status.phase,
        RuntimePhase::Connecting | RuntimePhase::Running
    ) {
        return Err("托管运行中时不能执行连接测试，请先停止托管。".into());
    }

    let settings_store = settings_store(&app).map_err(err_to_string)?;
    let settings = load_saved_settings(&settings_store).map_err(err_to_string)?;
    validate_company_profile(&settings.company_profile).map_err(err_to_string)?;
    validate_user_profile(&settings.user_profile).map_err(err_to_string)?;
    validate_target_profile(&settings.target_profile).map_err(err_to_string)?;

    let secret_store = token_store(&app).map_err(err_to_string)?;
    let token = load_saved_token(&secret_store).map_err(err_to_string)?;
    let ssh_password = ssh_password_store(&app)
        .and_then(|store| store.get_secret())
        .map_err(err_to_string)?;
    let runtime_target = default_runtime_target(current_platform().map_err(err_to_string)?);
    let app_handle = app.clone();
    let target_profile = settings.target_profile.clone();
    let company_profile = settings.company_profile.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        run_connection_test(
            &app_handle,
            runtime_target,
            &target_profile,
            &company_profile,
            &token,
            ssh_password.as_deref(),
        )
        .map_err(err_to_string)
    })
    .await
    .map_err(err_to_string)?
}

#[tauri::command]
pub fn start_runtime(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let guard = PortTaskGuard::acquire(state.port_task_active.clone()).map_err(err_to_string)?;
    let runtime_status = snapshot_status(&state.runtime);
    if matches!(
        runtime_status.phase,
        RuntimePhase::Connecting | RuntimePhase::Running
    ) {
        return Err("托管已在运行或正在连接，请稍候。".into());
    }

    let settings_store = settings_store(&app).map_err(err_to_string)?;
    let settings = load_saved_settings(&settings_store).map_err(err_to_string)?;
    validate_company_profile(&settings.company_profile).map_err(err_to_string)?;
    validate_user_profile(&settings.user_profile).map_err(err_to_string)?;
    validate_target_profile(&settings.target_profile).map_err(err_to_string)?;

    let secret_store = token_store(&app).map_err(err_to_string)?;
    let token = load_saved_token(&secret_store).map_err(err_to_string)?;
    let ssh_password = ssh_password_store(&app)
        .and_then(|store| store.get_secret())
        .map_err(err_to_string)?;
    let runtime_target = default_runtime_target(current_platform().map_err(err_to_string)?);
    let shared_runtime = state.runtime.clone();
    let app_handle = app.clone();
    let target_profile = settings.target_profile.clone();
    let company_profile = settings.company_profile.clone();
    let user_profile = settings.user_profile.clone();
    let token_clone = token.clone();

    thread::spawn(move || {
        let _guard = guard;
        let _ = start_runtime_processes(
            shared_runtime,
            app_handle,
            runtime_target,
            &target_profile,
            &company_profile,
            &user_profile,
            &token_clone,
            ssh_password.as_deref(),
        );
    });

    let status = RuntimeStatus {
        phase: RuntimePhase::Connecting,
        ssh_connected: false,
        node_connected: false,
        gateway_connected: false,
        last_error: None,
    };
    update_cached_runtime_status(&state, &status);
    let _ = app_menu::refresh_status_menu_from_state(&app);

    Ok(status)
}

#[tauri::command]
pub fn stop_runtime(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let status =
        stop_runtime_processes(state.runtime.clone(), app.clone()).map_err(err_to_string)?;
    update_cached_runtime_status(&state, &status);
    let _ = app_menu::refresh_status_menu_from_state(&app);
    Ok(status)
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    Ok(snapshot_status(&state.runtime))
}

#[tauri::command]
pub fn stream_logs(state: State<'_, AppState>) -> Result<Vec<LogEntry>, String> {
    Ok(snapshot_logs(&state.runtime))
}

fn detect_environment_inner(
    app: &AppHandle,
    runtime: &SharedRuntimeState,
) -> Result<EnvironmentSnapshot> {
    let settings = settings_store(app).and_then(|store| store.load())?;
    let platform = current_platform()?;
    let target_profile = settings
        .as_ref()
        .map(|saved| saved.target_profile.clone())
        .unwrap_or_default();
    let runtime_target = default_runtime_target(platform);
    let resolved = resolve_environment(runtime_target, &target_profile);
    let token_state = inspect_token_state(token_store(app).and_then(|store| store.get_secret()));

    Ok(build_environment_snapshot(
        env::consts::OS,
        runtime_target,
        settings,
        snapshot_status(runtime),
        resolved,
        token_state,
    ))
}

fn refresh_environment_snapshot(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<EnvironmentSnapshot> {
    let snapshot = detect_environment_inner(app, &state.runtime)?;
    {
        let mut guard = state
            .environment_cache
            .lock()
            .expect("environment cache mutex poisoned");
        *guard = Some(snapshot.clone());
    }
    let _ = app_menu::refresh_status_menu_from_state(app);
    Ok(snapshot)
}

fn update_cached_runtime_status(state: &State<'_, AppState>, status: &RuntimeStatus) {
    let mut guard = state
        .environment_cache
        .lock()
        .expect("environment cache mutex poisoned");
    if let Some(snapshot) = guard.as_mut() {
        snapshot.runtime_status = status.clone();
    }
}

fn resolve_environment(
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> ResolvedEnvironment {
    let host_ssh_installed = command_available("ssh");
    let target_ssh_installed =
        target_command_available(runtime_target.clone(), target_profile, "ssh");
    let openclaw_installed =
        target_command_available(runtime_target.clone(), target_profile, "openclaw");
    let openclaw_version = openclaw_installed
        .then(|| read_openclaw_version(runtime_target.clone(), target_profile))
        .flatten();
    let latest_openclaw_version =
        read_latest_openclaw_version(runtime_target.clone(), target_profile);
    let update_available = openclaw_version
        .as_deref()
        .zip(latest_openclaw_version.as_deref())
        .map(|(installed, latest)| compare_versions(installed, latest))
        .unwrap_or(false);
    let wsl_status = match runtime_target {
        RuntimeTarget::MacNative => None,
        RuntimeTarget::WindowsWsl => Some(detect_wsl_status(target_profile)),
    };

    ResolvedEnvironment {
        runtime_target,
        host_ssh_installed,
        target_ssh_installed,
        openclaw_installed,
        openclaw_version,
        latest_openclaw_version,
        update_available,
        wsl_status,
    }
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
    runtime_target: RuntimeTarget,
    settings: Option<PersistedSettings>,
    mut runtime_status: RuntimeStatus,
    resolved: ResolvedEnvironment,
    token_state: TokenState,
) -> EnvironmentSnapshot {
    let has_saved_profile = settings
        .as_ref()
        .map(saved_settings_are_complete)
        .unwrap_or(false);

    if !resolved.openclaw_installed {
        runtime_status.phase = RuntimePhase::InstallNeeded;
    } else if has_saved_profile && matches!(runtime_status.phase, RuntimePhase::Checking) {
        runtime_status.phase = RuntimePhase::Configured;
    }

    EnvironmentSnapshot {
        os: os.to_string(),
        runtime_target,
        host_ssh_installed: resolved.host_ssh_installed,
        target_ssh_installed: resolved.target_ssh_installed,
        openclaw_installed: resolved.openclaw_installed,
        openclaw_version: resolved.openclaw_version,
        latest_openclaw_version: resolved.latest_openclaw_version,
        update_available: resolved.update_available,
        has_saved_profile,
        token_status: token_state.status,
        token_status_message: token_state.message,
        saved_settings: settings,
        runtime_status,
        install_recommendation: install_recommendation(
            runtime_target,
            resolved.host_ssh_installed,
            resolved.target_ssh_installed,
            resolved.wsl_status.as_ref(),
        ),
        wsl_status: resolved.wsl_status,
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
    validate_target_profile(&settings.target_profile)?;

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
            rollback_secret(token_store, previous_token.as_deref()).map_err(|rollback_error| {
                anyhow!("{save_error}; token 回滚失败: {rollback_error}")
            })?;
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

fn run_connection_test(
    app: &AppHandle,
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    company_profile: &CompanyProfile,
    token: &str,
    ssh_password: Option<&str>,
) -> Result<ConnectionTestResult> {
    emit_connection_test_event(
        app,
        ConnectionTestStep::SshTunnel,
        ConnectionTestEventStatus::Running,
        format!("正在建立到 {} 的临时 SSH 隧道", company_profile.ssh_host),
    );

    let ssh_command = build_runtime_ssh_command(
        runtime_target,
        target_profile,
        company_profile,
        ssh_password,
    )?;
    let mut ssh_child = spawn_command_spec(&ssh_command).context("无法启动 SSH 隧道")?;

    let result: Result<ConnectionTestResult> = (|| {
        thread::sleep(Duration::from_secs(2));
        ensure_child_alive(&mut ssh_child, "SSH 隧道")?;
        wait_for_local_port_ready(
            company_profile.local_port,
            CONNECTION_TEST_TUNNEL_READY_TIMEOUT,
            CONNECTION_TEST_TUNNEL_POLL_INTERVAL,
        )?;

        emit_connection_test_event(
            app,
            ConnectionTestStep::SshTunnel,
            ConnectionTestEventStatus::Success,
            "SSH 隧道已建立".to_string(),
        );
        emit_connection_test_event(
            app,
            ConnectionTestStep::GatewayProbe,
            ConnectionTestEventStatus::Running,
            "正在验证 Gateway 鉴权".to_string(),
        );

        let gateway_result = run_command_capture(&build_gateway_status_command(
            runtime_target,
            target_profile,
            company_profile,
            token,
            CONNECTION_TEST_TIMEOUT_MS,
        ))?;

        if gateway_result.success {
            emit_connection_test_event(
                app,
                ConnectionTestStep::GatewayProbe,
                ConnectionTestEventStatus::Success,
                "Gateway 鉴权通过".to_string(),
            );
            Ok(ConnectionTestResult {
                success: true,
                step: ConnectionTestStep::GatewayProbe,
                summary: "SSH 隧道已建立，Gateway 鉴权通过。".into(),
                stdout: gateway_result.stdout,
                stderr: gateway_result.stderr,
            })
        } else {
            emit_connection_test_event(
                app,
                ConnectionTestStep::GatewayProbe,
                ConnectionTestEventStatus::Error,
                "Gateway 鉴权失败，请检查 Token、SSH 转发和远程 Gateway 状态。".to_string(),
            );
            Ok(ConnectionTestResult {
                success: false,
                step: ConnectionTestStep::GatewayProbe,
                summary: "Gateway 鉴权失败，请检查 Token、SSH 转发和远程 Gateway 状态。".into(),
                stdout: gateway_result.stdout,
                stderr: gateway_result.stderr,
            })
        }
    })();

    let _ = kill_child(&mut ssh_child);

    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            emit_connection_test_event(
                app,
                ConnectionTestStep::SshTunnel,
                ConnectionTestEventStatus::Error,
                error.to_string(),
            );
            Ok(ConnectionTestResult {
                success: false,
                step: ConnectionTestStep::SshTunnel,
                summary: "SSH 隧道建立失败，请检查 SSH 登录信息和端口转发配置。".into(),
                stdout: String::new(),
                stderr: error.to_string(),
            })
        }
    }
}

fn build_runtime_ssh_command(
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    company_profile: &CompanyProfile,
    ssh_password: Option<&str>,
) -> Result<CommandSpec> {
    match runtime_target {
        RuntimeTarget::MacNative => {
            if let Some(password) = ssh_password {
                let askpass_program =
                    std::env::current_exe().context("无法定位 SSH 密码辅助程序")?;
                return Ok(build_native_ssh_command(
                    company_profile,
                    Some((askpass_program.to_string_lossy().as_ref(), password)),
                ));
            }

            Ok(build_native_ssh_command(company_profile, None))
        }
        RuntimeTarget::WindowsWsl => Ok(build_wsl_ssh_command(
            target_profile,
            company_profile,
            ssh_password,
        )),
    }
}

fn build_gateway_status_command(
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    company_profile: &CompanyProfile,
    token: &str,
    timeout_ms: u64,
) -> CommandSpec {
    match runtime_target {
        RuntimeTarget::MacNative => {
            build_native_gateway_status_command(company_profile, token, timeout_ms)
        }
        RuntimeTarget::WindowsWsl => {
            build_wsl_gateway_status_command(target_profile, company_profile, token, timeout_ms)
        }
    }
}

fn spawn_command_spec(spec: &CommandSpec) -> Result<Child> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in &spec.envs {
        command.env(key, value);
    }
    command
        .spawn()
        .with_context(|| format!("启动命令失败: {}", spec.program))
}

fn run_command_capture(spec: &CommandSpec) -> Result<CapturedOutput> {
    let output = Command::new(&spec.program)
        .args(&spec.args)
        .envs(spec.envs.iter().cloned())
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("执行命令失败: {}", spec.program))?;

    Ok(CapturedOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn ensure_child_alive(child: &mut Child, label: &str) -> Result<()> {
    if let Some(status) = child.try_wait()? {
        return Err(anyhow!("{label} 提前退出，状态码: {status}"));
    }
    Ok(())
}

fn kill_child(child: &mut Child) -> Result<()> {
    child.kill().context("结束临时连接进程失败")?;
    let _ = child.wait();
    Ok(())
}

fn wait_for_local_port_ready(port: u16, max_wait: Duration, poll_interval: Duration) -> Result<()> {
    let started = std::time::Instant::now();
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    loop {
        match TcpStream::connect_timeout(&address, poll_interval) {
            Ok(_) => return Ok(()),
            Err(error) if started.elapsed() >= max_wait => {
                return Err(anyhow!("本地 SSH 隧道端口 {} 未就绪: {}", port, error));
            }
            Err(_) => thread::sleep(poll_interval),
        }
    }
}

fn emit_connection_test_event(
    app: &AppHandle,
    step: ConnectionTestStep,
    status: ConnectionTestEventStatus,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "connection-test-event",
        ConnectionTestEvent {
            step,
            status,
            message: message.into(),
            timestamp_ms: timestamp_ms(),
        },
    );
}

struct CapturedOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn install_recommendation(
    runtime_target: RuntimeTarget,
    host_ssh_installed: bool,
    target_ssh_installed: bool,
    wsl_status: Option<&WslStatus>,
) -> String {
    match runtime_target {
        RuntimeTarget::MacNative => {
            if !host_ssh_installed {
                "当前未检测到 macOS 自带的 ssh 命令，请先修复系统 OpenSSH 后再继续。".into()
            } else {
                "推荐先使用官方安装脚本，失败后再回退到 npm / pnpm 全局安装。".into()
            }
        }
        RuntimeTarget::WindowsWsl => {
            if let Some(status) = wsl_status {
                if !status.ready {
                    return format!(
                        "Windows 将通过 WSL 运行 OpenClaw；当前 {} 尚未就绪，可由 BizClaw 自动引导安装。",
                        status.distro_name
                    );
                }
            }
            if !target_ssh_installed {
                "已检测到 WSL，但 Ubuntu 中缺少 OpenSSH，BizClaw 会在安装流程里补齐。".into()
            } else {
                "Windows 版本固定通过 WSL Ubuntu 托管运行 OpenClaw Node 与 SSH 隧道。".into()
            }
        }
    }
}

fn start_install_or_update_task(
    app: &AppHandle,
    state: &State<'_, AppState>,
    kind: OperationKind,
    request: InstallRequest,
    runtime_target: RuntimeTarget,
    target_profile: TargetProfile,
) -> Result<OperationTaskSnapshot> {
    let snapshot = start_task(&state.operation, kind, OperationStep::Detect)?;
    emit_operation_status(app, &snapshot);

    let app_handle = app.clone();
    let runtime_state = state.runtime.clone();
    let operation_state = state.operation.clone();
    let environment_cache = state.environment_cache.clone();

    thread::spawn(move || {
        let result = run_install_or_update(
            &app_handle,
            &operation_state,
            kind,
            request,
            runtime_target,
            target_profile,
        );

        let task_snapshot = match result {
            Ok(operation_result) => {
                let phase = if cancel_requested(&operation_state) {
                    OperationTaskPhase::Cancelled
                } else if operation_result.success {
                    OperationTaskPhase::Success
                } else {
                    OperationTaskPhase::Error
                };
                finish_task(&operation_state, phase, operation_result)
            }
            Err(error) => finish_task(
                &operation_state,
                if cancel_requested(&operation_state) {
                    OperationTaskPhase::Cancelled
                } else {
                    OperationTaskPhase::Error
                },
                failure_result(
                    kind,
                    OperationStep::Detect,
                    "task-error",
                    &error.to_string(),
                ),
            ),
        };

        emit_operation_status(&app_handle, &task_snapshot);

        if let Ok(snapshot) = detect_environment_inner(&app_handle, &runtime_state) {
            {
                let mut guard = environment_cache
                    .lock()
                    .expect("environment cache mutex poisoned");
                *guard = Some(snapshot.clone());
            }
            let _ = app_menu::refresh_status_menu_from_state(&app_handle);
        } else {
            let _ = app_menu::refresh_status_menu_from_state(&app_handle);
        }
    });

    Ok(snapshot)
}

fn emit_operation_status(app: &AppHandle, snapshot: &OperationTaskSnapshot) {
    let _ = app.emit("operation-status", snapshot.clone());
    let _ = app_menu::refresh_status_menu_from_state(app);
}

pub fn stop_operation_for_exit(app: &AppHandle) -> Result<()> {
    let state = app.state::<AppState>();
    clear_child(&state.operation);
    let _ = request_stop(&state.operation);
    Ok(())
}

fn run_install_or_update(
    app: &AppHandle,
    operation_state: &SharedOperationState,
    kind: OperationKind,
    request: InstallRequest,
    runtime_target: RuntimeTarget,
    target_profile: TargetProfile,
) -> Result<OperationResult> {
    let snapshot = update_step(operation_state, OperationStep::Detect);
    emit_operation_status(app, &snapshot);
    emit_operation_event(
        app,
        Some(operation_state),
        kind.clone(),
        OperationStep::Detect,
        OperationEventStatus::Running,
        OperationEventSource::System,
        "正在检测目标运行环境",
    );
    let mut resolved = resolve_environment(runtime_target.clone(), &target_profile);
    emit_operation_event(
        app,
        Some(operation_state),
        kind.clone(),
        OperationStep::Detect,
        OperationEventStatus::Success,
        OperationEventSource::System,
        "环境检测完成",
    );

    if let Some(result) =
        cancelled_result_if_requested(operation_state, kind, OperationStep::Detect)
    {
        return Ok(result);
    }

    if matches!(runtime_target, RuntimeTarget::MacNative) && !resolved.host_ssh_installed {
        return Ok(failure_result(
            kind,
            OperationStep::EnsureSsh,
            "missing-ssh",
            "当前未检测到 macOS 的 ssh 命令，请先修复系统 OpenSSH 后再重试。",
        ));
    }

    if matches!(runtime_target, RuntimeTarget::WindowsWsl) {
        let wsl_status = resolved
            .wsl_status
            .clone()
            .unwrap_or_else(|| detect_wsl_status(&target_profile));
        if !wsl_status.ready {
            let snapshot = update_step(operation_state, OperationStep::BootstrapWsl);
            emit_operation_status(app, &snapshot);
            let bootstrap = run_single_plan(
                app,
                operation_state,
                kind.clone(),
                OperationStep::BootstrapWsl,
                current_platform()?,
                request.allow_elevation,
                &wsl_bootstrap_plan(&target_profile),
            )?;
            if cancel_requested(operation_state) {
                return Ok(cancelled_result(
                    kind,
                    OperationStep::BootstrapWsl,
                    bootstrap.strategy,
                    bootstrap.stdout,
                    bootstrap.stderr,
                ));
            }
            let ready_after = detect_wsl_status(&target_profile);
            if !bootstrap.success || !ready_after.ready {
                let follow_up = if bootstrap.stdout.to_lowercase().contains("restart")
                    || bootstrap.stderr.to_lowercase().contains("restart")
                {
                    "WSL 安装已启动，请在 Windows 重启并完成 Ubuntu 初始化后重新打开 BizClaw。"
                } else {
                    "请完成 WSL / Ubuntu 初始化后，再回到 BizClaw 继续安装。"
                };
                return Ok(OperationResult {
                    kind,
                    strategy: bootstrap.strategy,
                    success: false,
                    step: OperationStep::BootstrapWsl,
                    stdout: bootstrap.stdout,
                    stderr: bootstrap.stderr,
                    needs_elevation: bootstrap.needs_elevation,
                    manual_url: MANUAL_INSTALL_URL.into(),
                    follow_up: follow_up.into(),
                });
            }
            resolved = resolve_environment(runtime_target.clone(), &target_profile);
        }
    }

    if matches!(runtime_target, RuntimeTarget::WindowsWsl) && !resolved.target_ssh_installed {
        let snapshot = update_step(operation_state, OperationStep::EnsureSsh);
        emit_operation_status(app, &snapshot);
        let ssh_result = run_single_plan(
            app,
            operation_state,
            kind.clone(),
            OperationStep::EnsureSsh,
            current_platform()?,
            request.allow_elevation,
            &wsl_ensure_ssh_plan(&target_profile),
        )?;
        if cancel_requested(operation_state) {
            return Ok(cancelled_result(
                kind,
                OperationStep::EnsureSsh,
                ssh_result.strategy,
                ssh_result.stdout,
                ssh_result.stderr,
            ));
        }
        if !ssh_result.success {
            return Ok(OperationResult {
                kind,
                strategy: ssh_result.strategy,
                success: false,
                step: OperationStep::EnsureSsh,
                stdout: ssh_result.stdout,
                stderr: ssh_result.stderr,
                needs_elevation: ssh_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up:
                    "请确认 Ubuntu 已完成初始化，且当前用户具备安装 openssh-client 的 sudo 权限。"
                        .into(),
            });
        }
        resolved = resolve_environment(runtime_target.clone(), &target_profile);
    }

    if matches!(kind, OperationKind::Update) {
        let snapshot = update_step(operation_state, OperationStep::CheckUpdate);
        emit_operation_status(app, &snapshot);
        emit_operation_event(
            app,
            Some(operation_state),
            kind.clone(),
            OperationStep::CheckUpdate,
            OperationEventStatus::Running,
            OperationEventSource::System,
            "正在获取最新 OpenClaw 版本信息",
        );
        if !resolved.openclaw_installed {
            return Ok(failure_result(
                kind,
                OperationStep::CheckUpdate,
                "missing-openclaw",
                "请先安装 OpenClaw，再执行更新。",
            ));
        }
        if !resolved.update_available {
            emit_operation_event(
                app,
                Some(operation_state),
                kind.clone(),
                OperationStep::CheckUpdate,
                OperationEventStatus::Success,
                OperationEventSource::System,
                "当前已是最新版本，无需更新",
            );
            return Ok(OperationResult {
                kind,
                strategy: "noop".into(),
                success: true,
                step: OperationStep::CheckUpdate,
                stdout: String::new(),
                stderr: String::new(),
                needs_elevation: false,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: "当前已是最新版本。".into(),
            });
        }
        emit_operation_event(
            app,
            Some(operation_state),
            kind.clone(),
            OperationStep::CheckUpdate,
            OperationEventStatus::Success,
            OperationEventSource::System,
            format!(
                "发现新版本 {}",
                resolved
                    .latest_openclaw_version
                    .clone()
                    .unwrap_or_else(|| "未知版本".into())
            ),
        );
        if let Some(result) =
            cancelled_result_if_requested(operation_state, kind, OperationStep::CheckUpdate)
        {
            return Ok(result);
        }
    } else if resolved.openclaw_installed {
        return Ok(OperationResult {
            kind,
            strategy: "skipped".into(),
            success: true,
            step: OperationStep::InstallOpenClaw,
            stdout: String::new(),
            stderr: String::new(),
            needs_elevation: false,
            manual_url: MANUAL_INSTALL_URL.into(),
            follow_up: "已检测到 OpenClaw，跳过安装。".into(),
        });
    }

    let platform = current_platform()?;
    let mut plans = Vec::new();
    if request.prefer_official {
        plans.push(official_install_plan_for_target(
            runtime_target.clone(),
            &target_profile,
        ));
    }
    plans.extend(fallback_install_plans_for_target(
        runtime_target.clone(),
        &target_profile,
        target_command_available(runtime_target.clone(), &target_profile, "npm"),
        target_command_available(runtime_target.clone(), &target_profile, "pnpm"),
    ));

    let step = if matches!(kind, OperationKind::Update) {
        OperationStep::UpdateOpenClaw
    } else {
        OperationStep::InstallOpenClaw
    };
    let snapshot = update_step(operation_state, step);
    emit_operation_status(app, &snapshot);

    let mut last_result = None;
    for plan in plans {
        if let Some(result) = cancelled_result_if_requested(operation_state, kind, step) {
            return Ok(result);
        }

        let result = run_single_plan(
            app,
            operation_state,
            kind.clone(),
            step.clone(),
            platform,
            request.allow_elevation,
            &plan,
        )?;
        let cancelled = cancel_requested(operation_state);
        if should_treat_plan_as_success(
            kind,
            cancelled,
            result.success,
            target_command_available(runtime_target.clone(), &target_profile, "openclaw"),
        ) {
            return Ok(OperationResult {
                kind,
                strategy: result.strategy,
                success: true,
                step,
                stdout: result.stdout,
                stderr: result.stderr,
                needs_elevation: result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: if matches!(kind, OperationKind::Update) {
                    "OpenClaw 更新完成，请重新检测版本或直接启动连接。".into()
                } else {
                    "OpenClaw 安装完成，请继续保存连接并启动托管。".into()
                },
            });
        }
        if cancelled {
            return Ok(cancelled_result(
                kind,
                step,
                result.strategy,
                result.stdout,
                result.stderr,
            ));
        }
        last_result = Some(result);
    }

    let result = last_result.unwrap_or_else(|| StreamedPlanResult {
        strategy: "unknown".into(),
        success: false,
        stdout: String::new(),
        stderr: "安装未执行。".into(),
        needs_elevation: false,
    });
    Ok(OperationResult {
        kind,
        strategy: result.strategy,
        success: false,
        step,
        stdout: result.stdout,
        stderr: result.stderr,
        needs_elevation: result.needs_elevation,
        manual_url: MANUAL_INSTALL_URL.into(),
        follow_up: "操作未完成，请查看输出后重试，或改用官方手动安装文档。".into(),
    })
}

fn should_treat_plan_as_success(
    kind: OperationKind,
    cancelled: bool,
    plan_succeeded: bool,
    openclaw_available: bool,
) -> bool {
    if cancelled {
        return false;
    }

    if plan_succeeded {
        return true;
    }

    matches!(kind, OperationKind::Install) && openclaw_available
}

fn cancelled_follow_up(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Install => "安装已停止。",
        OperationKind::Update => "更新已停止。",
        OperationKind::CheckUpdate => "检查更新已停止。",
    }
}

fn cancelled_result_if_requested(
    operation_state: &SharedOperationState,
    kind: OperationKind,
    step: OperationStep,
) -> Option<OperationResult> {
    cancel_requested(operation_state)
        .then(|| cancelled_result(kind, step, "cancelled", String::new(), String::new()))
}

fn cancelled_result(
    kind: OperationKind,
    step: OperationStep,
    strategy: impl Into<String>,
    stdout: String,
    stderr: String,
) -> OperationResult {
    OperationResult {
        kind,
        strategy: strategy.into(),
        success: false,
        step,
        stdout,
        stderr,
        needs_elevation: false,
        manual_url: MANUAL_INSTALL_URL.into(),
        follow_up: cancelled_follow_up(kind).into(),
    }
}

fn failure_result(
    kind: OperationKind,
    step: OperationStep,
    strategy: &str,
    follow_up: &str,
) -> OperationResult {
    OperationResult {
        kind,
        strategy: strategy.into(),
        success: false,
        step,
        stdout: String::new(),
        stderr: String::new(),
        needs_elevation: false,
        manual_url: MANUAL_INSTALL_URL.into(),
        follow_up: follow_up.into(),
    }
}

fn run_single_plan(
    app: &AppHandle,
    operation_state: &SharedOperationState,
    kind: OperationKind,
    step: OperationStep,
    platform: Platform,
    allow_elevation: bool,
    plan: &InstallPlan,
) -> Result<StreamedPlanResult> {
    let result = execute_plan_streaming(app, operation_state, kind.clone(), step.clone(), plan)?;
    if cancel_requested(operation_state)
        || result.success
        || !result.needs_elevation
        || !allow_elevation
    {
        return Ok(result);
    }

    if let Some(elevated_plan) = elevated_install_plan(plan, platform) {
        return execute_plan_streaming(app, operation_state, kind, step, &elevated_plan);
    }

    Ok(result)
}

fn execute_plan_streaming(
    app: &AppHandle,
    operation_state: &SharedOperationState,
    kind: OperationKind,
    step: OperationStep,
    plan: &InstallPlan,
) -> Result<StreamedPlanResult> {
    emit_operation_event(
        app,
        Some(operation_state),
        kind.clone(),
        step.clone(),
        OperationEventStatus::Running,
        OperationEventSource::System,
        format!("开始执行 {}", plan.strategy),
    );

    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in &plan.envs {
        command.env(key, value);
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("执行命令失败: {}", plan.program))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    attach_child(operation_state, child);
    let (tx, rx) = mpsc::channel();

    if let Some(stdout) = stdout {
        let stdout_tx = tx.clone();
        thread::spawn(move || read_stream(stdout, OperationEventSource::Stdout, stdout_tx));
    }

    if let Some(stderr) = stderr {
        let stderr_tx = tx.clone();
        thread::spawn(move || read_stream(stderr, OperationEventSource::Stderr, stderr_tx));
    }

    drop(tx);

    let mut stdout_buffer = String::new();
    let mut stderr_buffer = String::new();
    let mut exit_status = None;
    let mut cancelled = false;

    while exit_status.is_none() {
        while let Ok((source, message)) = rx.recv_timeout(Duration::from_millis(50)) {
            match source {
                OperationEventSource::Stdout => {
                    stdout_buffer.push_str(&message);
                    stdout_buffer.push('\n');
                }
                OperationEventSource::Stderr => {
                    stderr_buffer.push_str(&message);
                    stderr_buffer.push('\n');
                }
                OperationEventSource::System => {}
            }
            emit_operation_event(
                app,
                Some(operation_state),
                kind.clone(),
                step.clone(),
                OperationEventStatus::Log,
                source,
                message,
            );
        }

        if cancel_requested(operation_state) {
            cancelled = true;
            let _ = with_child_mut(operation_state, |child| child.kill());
        }

        exit_status = with_child_mut(operation_state, |child| child.try_wait())
            .transpose()?
            .flatten();
    }

    while let Ok((source, message)) = rx.try_recv() {
        match source {
            OperationEventSource::Stdout => {
                stdout_buffer.push_str(&message);
                stdout_buffer.push('\n');
            }
            OperationEventSource::Stderr => {
                stderr_buffer.push_str(&message);
                stderr_buffer.push('\n');
            }
            OperationEventSource::System => {}
        }
        emit_operation_event(
            app,
            Some(operation_state),
            kind.clone(),
            step.clone(),
            OperationEventStatus::Log,
            source,
            message,
        );
    }

    clear_child(operation_state);

    let merged = format!("{stdout_buffer}\n{stderr_buffer}");
    let status = exit_status.expect("operation exit status should be available");
    let success = status.success() && !cancelled;
    emit_operation_event(
        app,
        Some(operation_state),
        kind,
        step,
        if cancelled {
            OperationEventStatus::Cancelled
        } else if success {
            OperationEventStatus::Success
        } else {
            OperationEventStatus::Error
        },
        OperationEventSource::System,
        if cancelled {
            format!("{} 已停止", plan.strategy)
        } else if success {
            format!("{} 执行完成", plan.strategy)
        } else {
            format!("{} 执行失败，退出状态 {}", plan.strategy, status)
        },
    );

    Ok(StreamedPlanResult {
        strategy: plan.strategy.into(),
        success,
        stdout: stdout_buffer,
        stderr: stderr_buffer,
        needs_elevation: looks_like_permission_error(&merged),
    })
}

fn read_stream(
    reader: impl std::io::Read + Send + 'static,
    source: OperationEventSource,
    tx: mpsc::Sender<(OperationEventSource, String)>,
) {
    let reader = BufReader::new(reader);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = tx.send((source.clone(), trimmed.to_string()));
    }
}

fn emit_operation_event(
    app: &AppHandle,
    operation_state: Option<&SharedOperationState>,
    kind: OperationKind,
    step: OperationStep,
    status: OperationEventStatus,
    source: OperationEventSource,
    message: impl Into<String>,
) {
    let event = OperationEvent {
        kind,
        step,
        status,
        source,
        message: message.into(),
        timestamp_ms: timestamp_ms(),
    };
    if let Some(state) = operation_state {
        push_event(state, event.clone());
    }
    let _ = app.emit("operation-event", event);
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn err_to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use anyhow::{anyhow, Result};

    use super::{
        build_environment_snapshot, cancelled_follow_up, inspect_token_state, load_saved_token,
        persist_profile_atomic, should_treat_plan_as_success, ResolvedEnvironment, RuntimePhase,
        RuntimeStatus, SettingsStoreAccess, TokenState,
    };
    use crate::{
        secret_store::{MemorySecretStore, SecretStore},
        types::{
            CompanyProfile, OperationKind, PersistedSettings, RuntimeTarget, TargetProfile,
            TokenStatus, UserProfile, WslStatus,
        },
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
    fn environment_snapshot_keeps_openclaw_version_and_update_status() {
        let snapshot = build_environment_snapshot(
            "macos",
            RuntimeTarget::MacNative,
            Some(sample_settings()),
            RuntimeStatus::default(),
            ResolvedEnvironment {
                runtime_target: RuntimeTarget::MacNative,
                host_ssh_installed: true,
                target_ssh_installed: true,
                openclaw_installed: true,
                openclaw_version: Some("OpenClaw 2026.3.8".into()),
                latest_openclaw_version: Some("2026.3.9".into()),
                update_available: true,
                wsl_status: None,
            },
            TokenState {
                status: TokenStatus::Saved,
                message: None,
            },
        );

        assert_eq!(
            snapshot.openclaw_version.as_deref(),
            Some("OpenClaw 2026.3.8")
        );
        assert_eq!(
            snapshot.latest_openclaw_version.as_deref(),
            Some("2026.3.9")
        );
        assert!(snapshot.update_available);
        assert_eq!(snapshot.token_status, TokenStatus::Saved);
        assert_eq!(snapshot.runtime_status.phase, RuntimePhase::Configured);
    }

    #[test]
    fn environment_snapshot_includes_wsl_status_when_target_is_windows() {
        let snapshot = build_environment_snapshot(
            "windows",
            RuntimeTarget::WindowsWsl,
            Some(sample_settings()),
            RuntimeStatus::default(),
            ResolvedEnvironment {
                runtime_target: RuntimeTarget::WindowsWsl,
                host_ssh_installed: false,
                target_ssh_installed: false,
                openclaw_installed: false,
                openclaw_version: None,
                latest_openclaw_version: None,
                update_available: false,
                wsl_status: Some(WslStatus {
                    available: true,
                    distro_name: "Ubuntu".into(),
                    distro_installed: false,
                    ready: false,
                    needs_reboot: false,
                    message: Some("missing".into()),
                }),
            },
            TokenState {
                status: TokenStatus::Missing,
                message: None,
            },
        );

        assert_eq!(snapshot.runtime_target, RuntimeTarget::WindowsWsl);
        assert_eq!(
            snapshot
                .wsl_status
                .as_ref()
                .map(|status| status.distro_name.as_str()),
            Some("Ubuntu")
        );
        assert_eq!(snapshot.runtime_status.phase, RuntimePhase::InstallNeeded);
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
        assert_eq!(
            token_state.message.as_deref(),
            Some("token file unavailable")
        );
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
        assert_eq!(
            token_store.get_secret().unwrap().as_deref(),
            Some("old-token")
        );
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

    #[test]
    fn install_can_use_path_detection_as_fallback_success() {
        assert!(should_treat_plan_as_success(
            OperationKind::Install,
            false,
            false,
            true,
        ));
    }

    #[test]
    fn update_cannot_use_path_detection_as_fallback_success() {
        assert!(!should_treat_plan_as_success(
            OperationKind::Update,
            false,
            false,
            true,
        ));
    }

    #[test]
    fn cancelled_operations_never_report_success() {
        assert!(!should_treat_plan_as_success(
            OperationKind::Install,
            true,
            true,
            true,
        ));
        assert_eq!(cancelled_follow_up(OperationKind::Install), "安装已停止。");
        assert_eq!(cancelled_follow_up(OperationKind::Update), "更新已停止。");
    }

    fn sample_settings() -> PersistedSettings {
        PersistedSettings {
            company_profile: CompanyProfile {
                ssh_host: "127.0.0.1".into(),
                ssh_user: "root".into(),
                local_port: 18889,
                remote_bind_host: "127.0.0.1".into(),
                remote_bind_port: 18789,
            },
            user_profile: UserProfile {
                display_name: "Goya Mac".into(),
                auto_connect: true,
                run_in_background: true,
            },
            target_profile: TargetProfile::default(),
        }
    }
}
