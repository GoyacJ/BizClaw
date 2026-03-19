use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    process::{Child, Stdio},
    sync::mpsc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use tauri::{window::Color, AppHandle, Emitter, Manager, State, Theme};
use tauri_plugin_opener::OpenerExt;

use crate::{
    app_menu,
    config_store::{JsonSettingsStore, JsonUiPreferencesStore},
    install::{
        command_available, compare_versions, current_platform, detect_wsl_status,
        elevated_install_plan, install_plans_for_target, macos_ensure_ssh_plan,
        read_latest_openclaw_version, read_openclaw_version, resolve_runtime_target,
        should_retry_with_elevation, target_command_available, update_plans_for_target,
        verify_windows_git_installation, verify_windows_node_installation,
        verify_windows_openclaw_installation, verify_windows_ssh_installation,
        windows_local_git_ready, windows_local_git_version, windows_local_node_ready,
        windows_local_node_version, windows_local_openclaw_ready, windows_local_openclaw_version,
        windows_local_ssh_ready, windows_native_ensure_git_plan, windows_native_ensure_node_plan,
        windows_native_ensure_ssh_plan, wsl_bootstrap_plan, wsl_ensure_ssh_plan, InstallPlan,
        Platform, WindowsInstallVerification, MANUAL_INSTALL_URL, WINDOWS_NODE_MIN_MAJOR,
    },
    openclaw_management,
    operation_supervisor::{
        attach_child, cancel_requested, clear_child, finish_task, new_shared_operation_state,
        push_event, request_stop, snapshot_events, snapshot_task, start_task, update_step,
        with_child_mut, SharedOperationState,
    },
    process_exec::new_command,
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
        ChatMessage, ChatMessageRole, ChatMessageStatus, ChatSessionSummary,
        ClawHubSkillSearchResult, CompanyProfile, ConnectionTestEvent, ConnectionTestEventStatus,
        ConnectionTestResult, ConnectionTestStep, CreateLocalSkillRequest,
        CreateOpenClawAgentRequest, EnvironmentSnapshot, InstallClawHubSkillRequest,
        InstallRequest, LocalePreference, LogEntry, OpenClawAgentBinding, OpenClawAgentSummary,
        OpenClawSkillCheckReport, OpenClawSkillInfo, OpenClawSkillInventory, OperationEvent,
        OperationEventSource, OperationEventStatus, OperationKind, OperationRemediation,
        OperationRemediationKind, OperationResult, OperationStep, OperationTaskPhase,
        OperationTaskSnapshot, PersistedSettings, RuntimePhase, RuntimeStatus, RuntimeTarget,
        SearchClawHubSkillsRequest, SendChatMessageRequest, SendChatMessageResult, SupportUrlTarget, TargetProfile, ThemePreference, TokenStatus,
        UiPreferences, UpdateOpenClawAgentIdentityRequest, UserProfile, WindowsDiscovery,
        WindowsDiscoveryPhase, WindowsNativeDiscovery, WindowsWslDiscovery, WslStatus,
    },
    validation::{
        saved_settings_are_complete, validate_company_profile_with_locale,
        validate_target_profile_with_locale, validate_user_profile_with_locale,
    },
};

pub struct AppState {
    pub runtime: SharedRuntimeState,
    pub operation: SharedOperationState,
    pub port_task_active: Arc<AtomicBool>,
    pub environment_cache: Arc<Mutex<Option<EnvironmentSnapshot>>>,
    environment_probe_active: Arc<AtomicBool>,
    latest_openclaw_version_cache: Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
    chat_store: Arc<Mutex<ChatStore>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            runtime: new_shared_runtime_state(),
            operation: new_shared_operation_state(),
            port_task_active: Arc::new(AtomicBool::new(false)),
            environment_cache: Arc::new(Mutex::new(None)),
            environment_probe_active: Arc::new(AtomicBool::new(false)),
            latest_openclaw_version_cache: Arc::new(Mutex::new(None)),
            chat_store: Arc::new(Mutex::new(ChatStore::default())),
        }
    }
}

#[derive(Debug, Default)]
struct ChatStore {
    sessions: Vec<ChatSessionSummary>,
    messages_by_session_id: HashMap<String, Vec<ChatMessage>>,
}

const HOMEBREW_INSTALL_URL: &str = "https://brew.sh";
const LATEST_OPENCLAW_VERSION_CACHE_TTL_MS: u64 = 60_000;

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
    host_openclaw_installed: bool,
    target_ssh_installed: bool,
    wsl_openclaw_installed: bool,
    openclaw_installed: bool,
    openclaw_version: Option<String>,
    latest_openclaw_version: Option<String>,
    update_available: bool,
    wsl_status: Option<WslStatus>,
    windows_discovery: Option<WindowsDiscovery>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LatestOpenClawVersionCacheEntry {
    cache_key: String,
    version: Option<String>,
    checked_at_ms: u64,
}

#[derive(Debug)]
struct StreamedPlanResult {
    strategy: String,
    success: bool,
    exit_code: i32,
    stdout: String,
    stderr: String,
    needs_elevation: bool,
}

struct PortTaskGuard {
    flag: Arc<AtomicBool>,
}

impl PortTaskGuard {
    fn acquire(flag: Arc<AtomicBool>, locale: LocalePreference) -> Result<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                anyhow!(locale_text(
                    locale,
                    "当前有连接操作进行中，请稍候",
                    "Another connection action is already running. Please wait."
                ))
            })?;
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
const WINDOWS_INSTALL_VERIFICATION_TIMEOUT: Duration = Duration::from_secs(20);

fn locale_text(locale: LocalePreference, zh: &'static str, en: &'static str) -> &'static str {
    if matches!(locale, LocalePreference::EnUs) {
        en
    } else {
        zh
    }
}

fn locale_owned(locale: LocalePreference, zh: impl Into<String>, en: impl Into<String>) -> String {
    if matches!(locale, LocalePreference::EnUs) {
        en.into()
    } else {
        zh.into()
    }
}

fn install_target_for_request(
    platform: Platform,
    snapshot: &EnvironmentSnapshot,
    request: &InstallRequest,
) -> Result<RuntimeTarget> {
    if !matches!(platform, Platform::Windows) {
        return Ok(snapshot.runtime_target);
    }

    match request.windows_target {
        None => Ok(snapshot.runtime_target),
        Some(RuntimeTarget::WindowsNative) => Ok(RuntimeTarget::WindowsNative),
        Some(RuntimeTarget::WindowsWsl) => Ok(RuntimeTarget::WindowsWsl),
        Some(other) => Err(anyhow!("invalid windows install target: {:?}", other)),
    }
}

fn extract_installer_exit_code(result: &StreamedPlanResult) -> Option<String> {
    if result.exit_code != 0 {
        return Some(result.exit_code.to_string());
    }

    [result.stdout.as_str(), result.stderr.as_str()]
        .into_iter()
        .flat_map(str::lines)
        .find_map(|line| {
            line.split("exit code:")
                .nth(1)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn node_install_failure_follow_up(locale: LocalePreference, result: &StreamedPlanResult) -> String {
    if let Some(exit_code) = extract_installer_exit_code(result) {
        return locale_owned(
            locale,
            format!("Node.js installer finished with exit code {exit_code}. Check the output and try again."),
            format!("The Node.js installer finished with exit code {exit_code}. Check the output and try again."),
        );
    }

    locale_text(
        locale,
        "BizClaw could not install Node.js automatically. Check the output, then retry and allow the Windows elevation prompt to continue.",
        "BizClaw could not install Node.js automatically. Check the output, then retry and allow the Windows elevation prompt to continue.",
    )
    .into()
}

fn git_install_failure_follow_up(locale: LocalePreference, result: &StreamedPlanResult) -> String {
    if let Some(exit_code) = extract_installer_exit_code(result) {
        return locale_owned(
            locale,
            format!("Git installer finished with exit code {exit_code}. Check the output and try again."),
            format!("The Git installer finished with exit code {exit_code}. Check the output and try again."),
        );
    }

    locale_text(
        locale,
        "BizClaw could not install Git automatically. Check the output, then retry and allow the Windows elevation prompt to continue.",
        "BizClaw could not install Git automatically. Check the output, then retry and allow the Windows elevation prompt to continue.",
    )
    .into()
}

fn openssh_verification_follow_up(
    locale: LocalePreference,
    verification: &WindowsInstallVerification,
) -> String {
    match verification {
        WindowsInstallVerification::MissingExecutable => locale_text(
            locale,
            "The OpenSSH install step completed, but ssh.exe was not detected. Check whether Windows OpenSSH Client finished installing.",
            "The OpenSSH install step completed, but ssh.exe was not detected. Check whether Windows OpenSSH Client finished installing.",
        )
        .into(),
        WindowsInstallVerification::CommandFailed { path, details } => locale_owned(
            locale,
            format!(
                "OpenSSH was installed at {}, but running ssh -V failed: {}.",
                path.display(),
                details
            ),
            format!(
                "OpenSSH was installed at {}, but running ssh -V failed: {}.",
                path.display(),
                details
            ),
        ),
        WindowsInstallVerification::Verified { .. }
        | WindowsInstallVerification::VersionTooLow { .. } => locale_text(
            locale,
            "The OpenSSH install step completed, but verification still failed. Check the output and try again.",
            "The OpenSSH install step completed, but verification still failed. Check the output and try again.",
        )
        .into(),
    }
}

fn node_verification_follow_up(
    locale: LocalePreference,
    verification: &WindowsInstallVerification,
) -> String {
    match verification {
        WindowsInstallVerification::MissingExecutable => locale_text(
            locale,
            "The Node.js installer ran, but node.exe was not detected.",
            "The Node.js installer ran, but node.exe was not detected.",
        )
        .into(),
        WindowsInstallVerification::CommandFailed { path, details } => locale_owned(
            locale,
            format!(
                "The Node.js installer ran and {} was detected, but running --version failed: {}.",
                path.display(),
                details
            ),
            format!(
                "The Node.js installer ran and {} was detected, but running --version failed: {}.",
                path.display(),
                details
            ),
        ),
        WindowsInstallVerification::VersionTooLow {
            version,
            minimum_major,
            ..
        } => locale_owned(
            locale,
            format!(
                "The Node.js installer ran, but the detected version {} is still below the required {}+.",
                version, minimum_major
            ),
            format!(
                "The Node.js installer ran, but the detected version {} is still below the required {}+.",
                version, minimum_major
            ),
        ),
        WindowsInstallVerification::Verified { .. } => locale_text(
            locale,
            "The Node.js installer ran, but verification still failed.",
            "The Node.js installer ran, but verification still failed.",
        )
        .into(),
    }
}

fn git_verification_follow_up(
    locale: LocalePreference,
    verification: &WindowsInstallVerification,
) -> String {
    match verification {
        WindowsInstallVerification::MissingExecutable => locale_text(
            locale,
            "The Git installer ran, but git.exe was not detected.",
            "The Git installer ran, but git.exe was not detected.",
        )
        .into(),
        WindowsInstallVerification::CommandFailed { path, details } => locale_owned(
            locale,
            format!(
                "The Git installer ran and {} was detected, but running --version failed: {}.",
                path.display(),
                details
            ),
            format!(
                "The Git installer ran and {} was detected, but running --version failed: {}.",
                path.display(),
                details
            ),
        ),
        WindowsInstallVerification::VersionTooLow { version, .. } => locale_owned(
            locale,
            format!(
                "The Git installer ran, but the detected version still looks wrong: {}.",
                version
            ),
            format!(
                "The Git installer ran, but the detected version still looks wrong: {}.",
                version
            ),
        ),
        WindowsInstallVerification::Verified { .. } => locale_text(
            locale,
            "The Git installer ran, but verification still failed.",
            "The Git installer ran, but verification still failed.",
        )
        .into(),
    }
}

#[tauri::command]
pub fn detect_environment(
    app: AppHandle,
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<EnvironmentSnapshot, String> {
    let force_refresh = force.unwrap_or(false);
    match current_platform().map_err(err_to_string)? {
        Platform::Windows => {
            if force_refresh {
                refresh_environment_snapshot(&app, &state, false).map_err(err_to_string)
            } else {
                let snapshot = detect_environment_quick(&app, &state).map_err(err_to_string)?;
                spawn_environment_probe(&app, &state, false);
                Ok(snapshot)
            }
        }
        Platform::MacOs => {
            if force_refresh {
                return refresh_environment_snapshot(&app, &state, false).map_err(err_to_string);
            }

            if let Some(snapshot) = cached_environment_snapshot(&state) {
                spawn_environment_probe(&app, &state, false);
                Ok(snapshot)
            } else {
                refresh_environment_snapshot(&app, &state, false).map_err(err_to_string)
            }
        }
    }
}

#[tauri::command]
pub fn install_openclaw(
    app: AppHandle,
    state: State<'_, AppState>,
    request: InstallRequest,
) -> Result<OperationTaskSnapshot, String> {
    let snapshot = refresh_environment_snapshot(&app, &state, false).map_err(err_to_string)?;
    let target_profile = snapshot
        .saved_settings
        .as_ref()
        .map(|settings| settings.target_profile.clone())
        .unwrap_or_default();
    let runtime_target = install_target_for_request(
        current_platform().map_err(err_to_string)?,
        &snapshot,
        &request,
    )
    .map_err(err_to_string)?;
    start_install_or_update_task(
        &app,
        &state,
        OperationKind::Install,
        request,
        runtime_target,
        target_profile,
    )
    .map_err(err_to_string)
}

#[tauri::command]
pub fn check_openclaw_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OperationTaskSnapshot, String> {
    start_check_update_task(&app, &state).map_err(err_to_string)
}

#[tauri::command]
pub fn update_openclaw(
    app: AppHandle,
    state: State<'_, AppState>,
    request: InstallRequest,
) -> Result<OperationTaskSnapshot, String> {
    let snapshot = refresh_environment_snapshot(&app, &state, false).map_err(err_to_string)?;
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
    open_support_url(app, SupportUrlTarget::OpenClawManual)
}

#[tauri::command]
pub fn open_support_url(app: AppHandle, target: SupportUrlTarget) -> Result<(), String> {
    app.opener()
        .open_url(support_url(target), None::<&str>)
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
    let locale = current_locale(&app);
    let settings = PersistedSettings {
        company_profile,
        user_profile,
        target_profile,
    };
    let store = settings_store(&app).map_err(err_to_string)?;
    let token_store = token_store(&app).map_err(err_to_string)?;
    let ssh_password_store = ssh_password_store(&app).map_err(err_to_string)?;
    persist_profile_atomic(
        locale,
        &store,
        &token_store,
        &ssh_password_store,
        &settings,
        &token,
        &ssh_password,
    )
    .map_err(err_to_string)?;

    mark_configured(&state.runtime, &app);
    let _ = app_menu::refresh_status_menu_from_state(&app);
    Ok(settings)
}

#[tauri::command]
pub async fn list_openclaw_agents() -> Result<Vec<OpenClawAgentSummary>, String> {
    run_blocking_command(openclaw_management::list_agents).await
}

#[tauri::command]
pub fn list_chat_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSessionSummary>, String> {
    let store = state
        .chat_store
        .lock()
        .map_err(|_| "chat store mutex poisoned".to_string())?;
    Ok(store.sessions.clone())
}

#[tauri::command]
pub fn create_chat_session(
    state: State<'_, AppState>,
    request: Option<Value>,
) -> Result<ChatSessionSummary, String> {
    let now = timestamp_ms();
    let mut store = state
        .chat_store
        .lock()
        .map_err(|_| "chat store mutex poisoned".to_string())?;
    let title = request
        .and_then(|entry| match entry {
            Value::String(value) => Some(value),
            Value::Object(mut value) => value
                .remove("title")
                .and_then(|raw_title| raw_title.as_str().map(ToString::to_string)),
            _ => None,
        })
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .unwrap_or_else(|| "新会话".to_string());
    let session = ChatSessionSummary {
        id: format!("session-{now}"),
        title,
        updated_at: now,
        preview: None,
    };
    store.sessions.insert(0, session.clone());
    store
        .messages_by_session_id
        .insert(session.id.clone(), Vec::new());
    Ok(session)
}

#[tauri::command]
pub fn list_chat_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ChatMessage>, String> {
    let store = state
        .chat_store
        .lock()
        .map_err(|_| "chat store mutex poisoned".to_string())?;
    Ok(store
        .messages_by_session_id
        .get(&session_id)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub fn send_chat_message(
    state: State<'_, AppState>,
    request: SendChatMessageRequest,
) -> Result<SendChatMessageResult, String> {
    let content = request.content.trim();
    if content.is_empty() {
        return Err("message content cannot be empty".to_string());
    }

    let now = timestamp_ms();
    let mut store = state
        .chat_store
        .lock()
        .map_err(|_| "chat store mutex poisoned".to_string())?;

    if !store.sessions.iter().any(|session| session.id == request.session_id) {
        return Err("chat session not found".to_string());
    }

    let user_message = ChatMessage {
        id: format!("msg-user-{now}"),
        role: ChatMessageRole::User,
        content: content.to_string(),
        created_at: now,
        status: ChatMessageStatus::Done,
    };
    let assistant_message = ChatMessage {
        id: format!("msg-assistant-{}", now + 1),
        role: ChatMessageRole::Assistant,
        content: format!("已收到：{content}"),
        created_at: now + 1,
        status: ChatMessageStatus::Done,
    };

    let messages = store
        .messages_by_session_id
        .entry(request.session_id.clone())
        .or_insert_with(Vec::new);
    messages.push(user_message.clone());
    messages.push(assistant_message.clone());

    if let Some(index) = store
        .sessions
        .iter()
        .position(|session| session.id == request.session_id)
    {
        let mut session = store.sessions.remove(index);
        session.updated_at = assistant_message.created_at;
        session.preview = Some(assistant_message.content.clone());
        store.sessions.insert(0, session);
    }

    Ok(SendChatMessageResult {
        user_message,
        assistant_message,
    })
}

#[tauri::command]
pub async fn create_openclaw_agent(request: CreateOpenClawAgentRequest) -> Result<Value, String> {
    run_blocking_command(move || openclaw_management::create_agent(&request)).await
}

#[tauri::command]
pub async fn update_openclaw_agent_identity(
    request: UpdateOpenClawAgentIdentityRequest,
) -> Result<Value, String> {
    run_blocking_command(move || openclaw_management::update_agent_identity(&request)).await
}

#[tauri::command]
pub async fn delete_openclaw_agent(agent_id: String) -> Result<Value, String> {
    run_blocking_command(move || openclaw_management::delete_agent(&agent_id)).await
}

#[tauri::command]
pub async fn list_openclaw_agent_bindings(
    agent_id: Option<String>,
) -> Result<Vec<OpenClawAgentBinding>, String> {
    run_blocking_command(move || openclaw_management::list_agent_bindings(agent_id.as_deref()))
        .await
}

#[tauri::command]
pub async fn add_openclaw_agent_bindings(
    agent_id: String,
    bindings: Vec<String>,
) -> Result<Value, String> {
    run_blocking_command(move || openclaw_management::add_agent_bindings(&agent_id, &bindings))
        .await
}

#[tauri::command]
pub async fn remove_openclaw_agent_bindings(
    agent_id: String,
    bindings: Option<Vec<String>>,
    remove_all: Option<bool>,
) -> Result<Value, String> {
    let bindings = bindings.unwrap_or_default();
    run_blocking_command(move || {
        openclaw_management::remove_agent_bindings(
            &agent_id,
            &bindings,
            remove_all.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn list_openclaw_skills() -> Result<OpenClawSkillInventory, String> {
    run_blocking_command(openclaw_management::list_skills).await
}

#[tauri::command]
pub async fn check_openclaw_skills() -> Result<OpenClawSkillCheckReport, String> {
    run_blocking_command(openclaw_management::check_skills).await
}

#[tauri::command]
pub async fn get_openclaw_skill_info(name: String) -> Result<OpenClawSkillInfo, String> {
    run_blocking_command(move || openclaw_management::get_skill_info(&name)).await
}

#[tauri::command]
pub async fn search_clawhub_skills(
    request: SearchClawHubSkillsRequest,
) -> Result<Vec<ClawHubSkillSearchResult>, String> {
    run_blocking_command(move || openclaw_management::search_clawhub_skills(&request)).await
}

#[tauri::command]
pub async fn install_clawhub_skill(request: InstallClawHubSkillRequest) -> Result<Value, String> {
    run_blocking_command(move || openclaw_management::install_clawhub_skill(&request)).await
}

#[tauri::command]
pub async fn create_local_openclaw_skill(
    request: CreateLocalSkillRequest,
) -> Result<Value, String> {
    run_blocking_command(move || openclaw_management::create_local_skill(&request)).await
}

#[tauri::command]
pub async fn delete_local_openclaw_skill(name: String) -> Result<Value, String> {
    run_blocking_command(move || openclaw_management::delete_local_skill(&name)).await
}

#[tauri::command]
pub fn save_ui_preferences(
    app: AppHandle,
    state: State<'_, AppState>,
    preferences: UiPreferences,
) -> Result<UiPreferences, String> {
    let store = ui_preferences_store(&app).map_err(err_to_string)?;
    store.save(&preferences).map_err(err_to_string)?;
    update_cached_ui_preferences(&state, &preferences);
    sync_main_window_appearance(&app, &preferences).map_err(err_to_string)?;
    let _ = app_menu::refresh_status_menu_from_state(&app);
    Ok(preferences)
}

#[tauri::command]
pub async fn test_connection(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, String> {
    let locale = current_locale(&app);
    let guard =
        PortTaskGuard::acquire(state.port_task_active.clone(), locale).map_err(err_to_string)?;
    let runtime_status = snapshot_status(&state.runtime);
    if matches!(
        runtime_status.phase,
        RuntimePhase::Connecting | RuntimePhase::Running
    ) {
        return Err(if matches!(locale, LocalePreference::EnUs) {
            "Stop the hosted runtime before running a connection test.".into()
        } else {
            "托管运行中时不能执行连接测试，请先停止托管。".into()
        });
    }

    let settings_store = settings_store(&app).map_err(err_to_string)?;
    let settings = load_saved_settings(locale, &settings_store).map_err(err_to_string)?;
    validate_company_profile_with_locale(&settings.company_profile, locale)
        .map_err(err_to_string)?;
    validate_user_profile_with_locale(&settings.user_profile, locale).map_err(err_to_string)?;
    validate_target_profile_with_locale(&settings.target_profile, locale).map_err(err_to_string)?;

    let secret_store = token_store(&app).map_err(err_to_string)?;
    let token = load_saved_token(locale, &secret_store).map_err(err_to_string)?;
    let ssh_password = ssh_password_store(&app)
        .and_then(|store| store.get_secret())
        .map_err(err_to_string)?;
    let target_profile = settings.target_profile.clone();
    let runtime_target =
        resolve_runtime_target(current_platform().map_err(err_to_string)?, &target_profile);
    let app_handle = app.clone();
    let company_profile = settings.company_profile.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        run_connection_test(
            &app_handle,
            locale,
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
    let locale = current_locale(&app);
    let guard =
        PortTaskGuard::acquire(state.port_task_active.clone(), locale).map_err(err_to_string)?;
    let runtime_status = snapshot_status(&state.runtime);
    if matches!(
        runtime_status.phase,
        RuntimePhase::Connecting | RuntimePhase::Running
    ) {
        return Err(if matches!(locale, LocalePreference::EnUs) {
            "The hosted runtime is already running or still connecting.".into()
        } else {
            "托管已在运行或正在连接，请稍候。".into()
        });
    }

    let settings_store = settings_store(&app).map_err(err_to_string)?;
    let settings = load_saved_settings(locale, &settings_store).map_err(err_to_string)?;
    validate_company_profile_with_locale(&settings.company_profile, locale)
        .map_err(err_to_string)?;
    validate_user_profile_with_locale(&settings.user_profile, locale).map_err(err_to_string)?;
    validate_target_profile_with_locale(&settings.target_profile, locale).map_err(err_to_string)?;

    let secret_store = token_store(&app).map_err(err_to_string)?;
    let token = load_saved_token(locale, &secret_store).map_err(err_to_string)?;
    let ssh_password = ssh_password_store(&app)
        .and_then(|store| store.get_secret())
        .map_err(err_to_string)?;
    let target_profile = settings.target_profile.clone();
    let runtime_target =
        resolve_runtime_target(current_platform().map_err(err_to_string)?, &target_profile);
    let shared_runtime = state.runtime.clone();
    let app_handle = app.clone();
    let company_profile = settings.company_profile.clone();
    let user_profile = settings.user_profile.clone();
    let token_clone = token.clone();

    thread::spawn(move || {
        let _guard = guard;
        let _ = start_runtime_processes(
            shared_runtime,
            app_handle,
            locale,
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
    let status = stop_runtime_processes(state.runtime.clone(), app.clone(), current_locale(&app))
        .map_err(err_to_string)?;
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

async fn run_blocking_command<T, F>(handler: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || handler().map_err(err_to_string))
        .await
        .map_err(err_to_string)?
}

fn detect_environment_inner(
    app: &AppHandle,
    runtime: &SharedRuntimeState,
    include_remote_update_check: bool,
    latest_version_cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
) -> Result<EnvironmentSnapshot> {
    let settings = settings_store(app).and_then(|store| store.load())?;
    let ui_preferences = ui_preferences_store(app)
        .and_then(|store| store.load())
        .map(|value| value.unwrap_or_default())?;
    let platform = current_platform()?;
    let target_profile = settings
        .as_ref()
        .map(|saved| saved.target_profile.clone())
        .unwrap_or_default();
    let resolved = match platform {
        Platform::MacOs => resolve_environment(
            RuntimeTarget::MacNative,
            &target_profile,
            include_remote_update_check,
            latest_version_cache,
        ),
        Platform::Windows => resolved_environment_from_windows_discovery(
            &target_profile,
            include_remote_update_check,
            latest_version_cache,
            detect_windows_discovery(&target_profile),
        ),
    };
    let token_state = inspect_token_state(token_store(app).and_then(|store| store.get_secret()));

    Ok(build_environment_snapshot(
        env::consts::OS,
        resolved.runtime_target,
        settings,
        ui_preferences,
        snapshot_status(runtime),
        resolved,
        token_state,
    ))
}

fn refresh_environment_snapshot(
    app: &AppHandle,
    state: &State<'_, AppState>,
    include_remote_update_check: bool,
) -> Result<EnvironmentSnapshot> {
    let snapshot = detect_environment_inner(
        app,
        &state.runtime,
        include_remote_update_check,
        &state.latest_openclaw_version_cache,
    )?;
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

fn detect_environment_quick(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<EnvironmentSnapshot> {
    let settings = settings_store(app).and_then(|store| store.load())?;
    let ui_preferences = ui_preferences_store(app)
        .and_then(|store| store.load())
        .map(|value| value.unwrap_or_default())?;
    let target_profile = settings
        .as_ref()
        .map(|saved| saved.target_profile.clone())
        .unwrap_or_default();
    let token_state = inspect_token_state(token_store(app).and_then(|store| store.get_secret()));
    let resolved =
        quick_windows_resolved_environment(&target_profile, &state.latest_openclaw_version_cache);

    Ok(build_environment_snapshot(
        env::consts::OS,
        resolved.runtime_target,
        settings,
        ui_preferences,
        snapshot_status(&state.runtime),
        resolved,
        token_state,
    ))
}

fn cached_environment_snapshot(state: &State<'_, AppState>) -> Option<EnvironmentSnapshot> {
    state
        .environment_cache
        .lock()
        .expect("environment cache mutex poisoned")
        .clone()
}

fn spawn_environment_probe(
    app: &AppHandle,
    state: &State<'_, AppState>,
    include_remote_update_check: bool,
) {
    if state
        .environment_probe_active
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let app_handle = app.clone();
    let runtime_state = state.runtime.clone();
    let environment_cache = state.environment_cache.clone();
    let latest_version_cache = state.latest_openclaw_version_cache.clone();
    let environment_probe_active = state.environment_probe_active.clone();

    thread::spawn(move || {
        let snapshot = detect_environment_inner(
            &app_handle,
            &runtime_state,
            include_remote_update_check,
            &latest_version_cache,
        );

        if let Ok(snapshot) = snapshot {
            {
                let mut guard = environment_cache
                    .lock()
                    .expect("environment cache mutex poisoned");
                *guard = Some(snapshot.clone());
            }
            emit_environment_snapshot(&app_handle, &snapshot);
        }

        let _ = app_menu::refresh_status_menu_from_state(&app_handle);
        environment_probe_active.store(false, Ordering::Release);
    });
}

fn quick_windows_resolved_environment(
    target_profile: &TargetProfile,
    latest_version_cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
) -> ResolvedEnvironment {
    let host_ssh_installed = windows_local_ssh_ready();
    let host_openclaw_version = windows_local_openclaw_version();
    let host_openclaw_installed = host_openclaw_version.is_some();
    let latest_openclaw_version = read_cached_latest_openclaw_version(
        latest_version_cache,
        RuntimeTarget::WindowsNative,
        target_profile,
    )
    .flatten();
    let windows_discovery = WindowsDiscovery {
        phase: WindowsDiscoveryPhase::Pending,
        native: WindowsNativeDiscovery {
            ssh_installed: host_ssh_installed,
            node_installed: windows_local_node_ready(),
            node_version: windows_local_node_version(),
            git_installed: windows_local_git_ready(),
            git_version: windows_local_git_version(),
            openclaw_installed: host_openclaw_installed,
            openclaw_version: host_openclaw_version.clone(),
        },
        wsl: WindowsWslDiscovery::default(),
    };

    ResolvedEnvironment {
        runtime_target: RuntimeTarget::WindowsNative,
        host_ssh_installed,
        host_openclaw_installed,
        target_ssh_installed: host_ssh_installed,
        wsl_openclaw_installed: false,
        openclaw_installed: host_openclaw_installed,
        openclaw_version: host_openclaw_version.clone(),
        latest_openclaw_version: latest_openclaw_version.clone(),
        update_available: update_available_from_versions(
            host_openclaw_version.as_deref(),
            latest_openclaw_version.as_deref(),
        ),
        wsl_status: None,
        windows_discovery: Some(windows_discovery),
    }
}

fn detect_windows_discovery(target_profile: &TargetProfile) -> WindowsDiscovery {
    let native_openclaw_version = windows_local_openclaw_version();
    let wsl_status = detect_wsl_status(target_profile);
    let wsl_ssh_installed = wsl_status.ready
        && target_command_available(RuntimeTarget::WindowsWsl, target_profile, "ssh");
    let wsl_openclaw_version = if wsl_status.ready {
        read_openclaw_version(RuntimeTarget::WindowsWsl, target_profile)
    } else {
        None
    };

    WindowsDiscovery {
        phase: WindowsDiscoveryPhase::Ready,
        native: WindowsNativeDiscovery {
            ssh_installed: windows_local_ssh_ready(),
            node_installed: windows_local_node_ready(),
            node_version: windows_local_node_version(),
            git_installed: windows_local_git_ready(),
            git_version: windows_local_git_version(),
            openclaw_installed: native_openclaw_version.is_some(),
            openclaw_version: native_openclaw_version,
        },
        wsl: WindowsWslDiscovery {
            status: Some(wsl_status.clone()),
            ssh_installed: wsl_ssh_installed,
            openclaw_installed: wsl_openclaw_version.is_some(),
            openclaw_version: wsl_openclaw_version,
        },
    }
}

fn resolved_environment_from_windows_discovery(
    target_profile: &TargetProfile,
    include_remote_update_check: bool,
    latest_version_cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
    windows_discovery: WindowsDiscovery,
) -> ResolvedEnvironment {
    let runtime_target = if windows_discovery.native.openclaw_installed {
        RuntimeTarget::WindowsNative
    } else if windows_discovery.wsl.openclaw_installed {
        RuntimeTarget::WindowsWsl
    } else {
        RuntimeTarget::WindowsNative
    };
    let openclaw_version = match runtime_target {
        RuntimeTarget::WindowsNative => windows_discovery.native.openclaw_version.clone(),
        RuntimeTarget::WindowsWsl => windows_discovery.wsl.openclaw_version.clone(),
        RuntimeTarget::MacNative => None,
    };
    let latest_openclaw_version = if include_remote_update_check {
        read_latest_openclaw_version_cached(latest_version_cache, runtime_target, target_profile)
    } else {
        read_cached_latest_openclaw_version(latest_version_cache, runtime_target, target_profile)
            .flatten()
    };

    ResolvedEnvironment {
        runtime_target,
        host_ssh_installed: windows_discovery.native.ssh_installed,
        host_openclaw_installed: windows_discovery.native.openclaw_installed,
        target_ssh_installed: match runtime_target {
            RuntimeTarget::WindowsNative => windows_discovery.native.ssh_installed,
            RuntimeTarget::WindowsWsl => windows_discovery.wsl.ssh_installed,
            RuntimeTarget::MacNative => false,
        },
        wsl_openclaw_installed: windows_discovery.wsl.openclaw_installed,
        openclaw_installed: openclaw_version.is_some(),
        openclaw_version: openclaw_version.clone(),
        latest_openclaw_version: latest_openclaw_version.clone(),
        update_available: update_available_from_versions(
            openclaw_version.as_deref(),
            latest_openclaw_version.as_deref(),
        ),
        wsl_status: matches!(runtime_target, RuntimeTarget::WindowsWsl)
            .then(|| windows_discovery.wsl.status.clone())
            .flatten(),
        windows_discovery: Some(windows_discovery),
    }
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

fn update_cached_ui_preferences(state: &State<'_, AppState>, preferences: &UiPreferences) {
    let mut guard = state
        .environment_cache
        .lock()
        .expect("environment cache mutex poisoned");
    if let Some(snapshot) = guard.as_mut() {
        apply_ui_preferences_to_snapshot(snapshot, preferences.clone());
    }
}

fn resolve_environment(
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    include_remote_update_check: bool,
    latest_version_cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
) -> ResolvedEnvironment {
    let host_ssh_installed = match runtime_target {
        RuntimeTarget::WindowsNative | RuntimeTarget::WindowsWsl => windows_local_ssh_ready(),
        RuntimeTarget::MacNative => command_available("ssh"),
    };
    let host_openclaw_installed = match runtime_target {
        RuntimeTarget::WindowsNative | RuntimeTarget::WindowsWsl => windows_local_openclaw_ready(),
        RuntimeTarget::MacNative => command_available("openclaw"),
    };
    let windows_wsl_status = if matches!(runtime_target, RuntimeTarget::WindowsWsl)
        || (!host_openclaw_installed && matches!(runtime_target, RuntimeTarget::WindowsNative))
    {
        Some(detect_wsl_status(target_profile))
    } else {
        None
    };
    let wsl_openclaw_installed = windows_wsl_status
        .as_ref()
        .map(|status| {
            status.ready
                && target_command_available(RuntimeTarget::WindowsWsl, target_profile, "openclaw")
        })
        .unwrap_or(false);
    let target_ssh_installed = match runtime_target {
        RuntimeTarget::MacNative => host_ssh_installed,
        RuntimeTarget::WindowsNative => windows_local_ssh_ready(),
        RuntimeTarget::WindowsWsl => {
            target_command_available(RuntimeTarget::WindowsWsl, target_profile, "ssh")
        }
    };
    let openclaw_installed = match runtime_target {
        RuntimeTarget::MacNative | RuntimeTarget::WindowsNative => host_openclaw_installed,
        RuntimeTarget::WindowsWsl => wsl_openclaw_installed,
    };
    let openclaw_version = openclaw_installed
        .then(|| read_openclaw_version(runtime_target.clone(), target_profile))
        .flatten();
    let latest_openclaw_version = if include_remote_update_check {
        read_latest_openclaw_version_cached(
            latest_version_cache,
            runtime_target.clone(),
            target_profile,
        )
    } else {
        read_cached_latest_openclaw_version(
            latest_version_cache,
            runtime_target.clone(),
            target_profile,
        )
        .flatten()
    };
    let update_available = update_available_from_versions(
        openclaw_version.as_deref(),
        latest_openclaw_version.as_deref(),
    );
    let wsl_status = match runtime_target {
        RuntimeTarget::MacNative | RuntimeTarget::WindowsNative => None,
        RuntimeTarget::WindowsWsl => windows_wsl_status,
    };

    ResolvedEnvironment {
        runtime_target,
        host_ssh_installed,
        host_openclaw_installed,
        target_ssh_installed,
        wsl_openclaw_installed,
        openclaw_installed,
        openclaw_version,
        latest_openclaw_version,
        update_available,
        wsl_status,
        windows_discovery: None,
    }
}

fn read_latest_openclaw_version_cached(
    cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> Option<String> {
    let cache_key = latest_openclaw_version_cache_key(&runtime_target, target_profile);
    with_cached_latest_openclaw_version(cache, &cache_key, timestamp_ms(), || {
        read_latest_openclaw_version(runtime_target, target_profile)
    })
}

fn read_cached_latest_openclaw_version(
    cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> Option<Option<String>> {
    let cache_key = latest_openclaw_version_cache_key(&runtime_target, target_profile);
    cached_latest_openclaw_version(cache, &cache_key, timestamp_ms())
}

fn latest_openclaw_version_cache_key(
    runtime_target: &RuntimeTarget,
    target_profile: &TargetProfile,
) -> String {
    match runtime_target {
        RuntimeTarget::MacNative => "macNative".into(),
        RuntimeTarget::WindowsNative => "windowsNative".into(),
        RuntimeTarget::WindowsWsl => format!("windowsWsl:{}", target_profile.wsl_distro),
    }
}

fn with_cached_latest_openclaw_version(
    cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
    cache_key: &str,
    now_ms: u64,
    fetch: impl FnOnce() -> Option<String>,
) -> Option<String> {
    {
        let guard = cache.lock().expect("latest version cache mutex poisoned");
        if let Some(entry) = guard.as_ref() {
            if entry.cache_key == cache_key
                && now_ms.saturating_sub(entry.checked_at_ms) < LATEST_OPENCLAW_VERSION_CACHE_TTL_MS
            {
                return entry.version.clone();
            }
        }
    }

    let version = fetch();
    let mut guard = cache.lock().expect("latest version cache mutex poisoned");
    *guard = Some(LatestOpenClawVersionCacheEntry {
        cache_key: cache_key.into(),
        version: version.clone(),
        checked_at_ms: now_ms,
    });
    version
}

fn cached_latest_openclaw_version(
    cache: &Arc<Mutex<Option<LatestOpenClawVersionCacheEntry>>>,
    cache_key: &str,
    now_ms: u64,
) -> Option<Option<String>> {
    let guard = cache.lock().expect("latest version cache mutex poisoned");
    guard.as_ref().and_then(|entry| {
        (entry.cache_key == cache_key
            && now_ms.saturating_sub(entry.checked_at_ms) < LATEST_OPENCLAW_VERSION_CACHE_TTL_MS)
            .then(|| entry.version.clone())
    })
}

fn update_available_from_versions(
    openclaw_version: Option<&str>,
    latest_openclaw_version: Option<&str>,
) -> bool {
    openclaw_version
        .zip(latest_openclaw_version)
        .map(|(installed, latest)| compare_versions(installed, latest))
        .unwrap_or(false)
}

fn settings_store(app: &AppHandle) -> Result<JsonSettingsStore> {
    let path = app
        .path()
        .app_data_dir()
        .context("Failed to resolve the app data directory")?
        .join("settings.json");
    Ok(JsonSettingsStore::new(path))
}

fn token_store(app: &AppHandle) -> Result<LocalSecretStore> {
    let path = app
        .path()
        .app_data_dir()
        .context("Failed to resolve the app data directory")?
        .join("gateway-token.txt");
    Ok(LocalSecretStore::new(path))
}

fn current_locale(app: &AppHandle) -> LocalePreference {
    load_ui_preferences(app)
        .ok()
        .map(|preferences| preferences.locale)
        .unwrap_or_default()
}

fn support_url(target: SupportUrlTarget) -> &'static str {
    match target {
        SupportUrlTarget::OpenClawManual => MANUAL_INSTALL_URL,
        SupportUrlTarget::HomebrewInstall => HOMEBREW_INSTALL_URL,
    }
}

fn ui_preferences_store(app: &AppHandle) -> Result<JsonUiPreferencesStore> {
    let path = app
        .path()
        .app_data_dir()
        .context("Failed to resolve the app data directory")?
        .join("ui-preferences.json");
    Ok(JsonUiPreferencesStore::new(path))
}

pub(crate) fn load_ui_preferences(app: &AppHandle) -> Result<UiPreferences> {
    ui_preferences_store(app)
        .and_then(|store| store.load())
        .map(|value| value.unwrap_or_default())
}

pub(crate) fn sync_main_window_appearance(
    app: &AppHandle,
    preferences: &UiPreferences,
) -> Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    window
        .set_theme(window_theme_for_preference(preferences.theme))
        .context("Failed to sync the native window theme")?;
    window
        .set_background_color(window_background_color_for_theme(preferences.theme))
        .context("Failed to sync the native window background color")?;

    Ok(())
}

fn ssh_password_store(app: &AppHandle) -> Result<LocalSecretStore> {
    let path = app
        .path()
        .app_data_dir()
        .context("Failed to resolve the app data directory")?
        .join("ssh-password.txt");
    Ok(LocalSecretStore::new(path))
}

fn build_environment_snapshot(
    os: &str,
    runtime_target: RuntimeTarget,
    settings: Option<PersistedSettings>,
    ui_preferences: UiPreferences,
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
        host_openclaw_installed: resolved.host_openclaw_installed,
        target_ssh_installed: resolved.target_ssh_installed,
        openclaw_installed: resolved.openclaw_installed,
        openclaw_version: resolved.openclaw_version,
        latest_openclaw_version: resolved.latest_openclaw_version,
        update_available: resolved.update_available,
        wsl_openclaw_installed: resolved.wsl_openclaw_installed,
        has_saved_profile,
        token_status: token_state.status,
        token_status_message: token_state.message,
        ui_preferences: ui_preferences.clone(),
        saved_settings: settings,
        runtime_status,
        install_recommendation: install_recommendation(
            ui_preferences.locale,
            runtime_target,
            resolved.host_ssh_installed,
            resolved.host_openclaw_installed,
            resolved.target_ssh_installed,
            resolved.wsl_openclaw_installed,
            resolved.wsl_status.as_ref(),
            resolved.windows_discovery.as_ref(),
        ),
        wsl_status: resolved.wsl_status,
        windows_discovery: resolved.windows_discovery,
    }
}

fn apply_ui_preferences_to_snapshot(
    snapshot: &mut EnvironmentSnapshot,
    preferences: UiPreferences,
) {
    snapshot.ui_preferences = preferences.clone();
    snapshot.install_recommendation = install_recommendation(
        preferences.locale,
        snapshot.runtime_target,
        snapshot.host_ssh_installed,
        snapshot.host_openclaw_installed,
        snapshot.target_ssh_installed,
        snapshot.wsl_openclaw_installed,
        snapshot.wsl_status.as_ref(),
        snapshot.windows_discovery.as_ref(),
    );
}

fn window_theme_for_preference(theme: ThemePreference) -> Option<Theme> {
    match theme {
        ThemePreference::Light => Some(Theme::Light),
        ThemePreference::Dark => Some(Theme::Dark),
        ThemePreference::System => None,
    }
}

fn window_background_color_for_theme(theme: ThemePreference) -> Option<Color> {
    match theme {
        ThemePreference::Light => Some(Color(245, 247, 251, 255)),
        ThemePreference::Dark => Some(Color(11, 18, 32, 255)),
        ThemePreference::System => None,
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
    locale: LocalePreference,
    settings_store: &impl SettingsStoreAccess,
    token_store: &impl SecretStore,
    ssh_password_store: &impl SecretStore,
    settings: &PersistedSettings,
    token: &str,
    ssh_password: &str,
) -> Result<()> {
    validate_company_profile_with_locale(&settings.company_profile, locale)?;
    validate_user_profile_with_locale(&settings.user_profile, locale)?;
    validate_target_profile_with_locale(&settings.target_profile, locale)?;

    let previous_token = token_store.get_secret()?;
    let previous_ssh_password = ssh_password_store.get_secret()?;
    let trimmed_token = token.trim();
    let trimmed_ssh_password = ssh_password.trim();
    let should_write_new_token = !trimmed_token.is_empty();
    let should_write_new_ssh_password = !trimmed_ssh_password.is_empty();

    if should_write_new_token {
        token_store.set_secret(trimmed_token)?;
    } else if previous_token.is_none() {
        return Err(anyhow!(locale_text(
            locale,
            "Token 不能为空",
            "Token cannot be empty"
        )));
    }

    if should_write_new_ssh_password {
        if let Err(save_error) = ssh_password_store.set_secret(trimmed_ssh_password) {
            if should_write_new_token {
                rollback_secret(token_store, previous_token.as_deref()).map_err(
                    |rollback_error| {
                        anyhow!(
                            "{}",
                            locale_owned(
                                locale,
                                format!("{save_error}; token 回滚失败: {rollback_error}"),
                                format!("{save_error}; token rollback failed: {rollback_error}")
                            )
                        )
                    },
                )?;
            }
            return Err(save_error);
        }
    }

    if let Err(save_error) = settings_store.save(settings) {
        if should_write_new_token {
            rollback_secret(token_store, previous_token.as_deref()).map_err(|rollback_error| {
                anyhow!(
                    "{}",
                    locale_owned(
                        locale,
                        format!("{save_error}; token 回滚失败: {rollback_error}"),
                        format!("{save_error}; token rollback failed: {rollback_error}")
                    )
                )
            })?;
        }
        if should_write_new_ssh_password {
            rollback_secret(ssh_password_store, previous_ssh_password.as_deref()).map_err(
                |rollback_error| {
                    anyhow!(
                        "{}",
                        locale_owned(
                            locale,
                            format!("{save_error}; SSH 密码回滚失败: {rollback_error}"),
                            format!("{save_error}; SSH password rollback failed: {rollback_error}")
                        )
                    )
                },
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

fn load_saved_settings(
    locale: LocalePreference,
    settings_store: &impl SettingsStoreAccess,
) -> Result<PersistedSettings> {
    settings_store.load()?.ok_or_else(|| {
        if matches!(locale, LocalePreference::EnUs) {
            anyhow!("BizClaw connection settings have not been saved yet")
        } else {
            anyhow!("尚未保存 BizClaw 连接配置")
        }
    })
}

fn load_saved_token(locale: LocalePreference, secret_store: &impl SecretStore) -> Result<String> {
    secret_store.get_secret()?.ok_or_else(|| {
        if matches!(locale, LocalePreference::EnUs) {
            anyhow!("OPENCLAW_GATEWAY_TOKEN has not been saved yet")
        } else {
            anyhow!("尚未保存 OPENCLAW_GATEWAY_TOKEN")
        }
    })
}

fn run_connection_test(
    app: &AppHandle,
    locale: LocalePreference,
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
        locale_owned(
            locale,
            format!("正在建立到 {} 的临时 SSH 隧道", company_profile.ssh_host),
            format!(
                "Creating a temporary SSH tunnel to {}",
                company_profile.ssh_host
            ),
        ),
    );

    let ssh_command = build_runtime_ssh_command(
        locale,
        runtime_target,
        target_profile,
        company_profile,
        ssh_password,
    )?;
    let mut ssh_child = spawn_command_spec(&ssh_command, locale).context(locale_text(
        locale,
        "无法启动 SSH 隧道",
        "Failed to start the SSH tunnel",
    ))?;

    let result: Result<ConnectionTestResult> = (|| {
        thread::sleep(Duration::from_secs(2));
        ensure_child_alive(
            &mut ssh_child,
            locale,
            locale_text(locale, "SSH 隧道", "SSH tunnel"),
        )?;
        wait_for_local_port_ready(
            locale,
            company_profile.local_port,
            CONNECTION_TEST_TUNNEL_READY_TIMEOUT,
            CONNECTION_TEST_TUNNEL_POLL_INTERVAL,
        )?;

        emit_connection_test_event(
            app,
            ConnectionTestStep::SshTunnel,
            ConnectionTestEventStatus::Success,
            locale_text(locale, "SSH 隧道已建立", "SSH tunnel established").to_string(),
        );
        emit_connection_test_event(
            app,
            ConnectionTestStep::GatewayProbe,
            ConnectionTestEventStatus::Running,
            locale_text(
                locale,
                "正在验证 Gateway 鉴权",
                "Validating Gateway authentication",
            )
            .to_string(),
        );

        let gateway_result = run_command_capture(
            &build_gateway_status_command(
                runtime_target,
                target_profile,
                company_profile,
                token,
                CONNECTION_TEST_TIMEOUT_MS,
            ),
            locale,
        )
        .map_err(|error| {
            emit_connection_test_event(
                app,
                ConnectionTestStep::GatewayProbe,
                ConnectionTestEventStatus::Error,
                error.to_string(),
            );
            error
        });

        let gateway_result = match gateway_result {
            Ok(result) => result,
            Err(error) => {
                return Ok(gateway_probe_failure_result(
                    locale,
                    String::new(),
                    error.to_string(),
                ));
            }
        };

        let gateway_probe = evaluate_gateway_probe(&gateway_result);

        if gateway_probe.success {
            emit_connection_test_event(
                app,
                ConnectionTestStep::GatewayProbe,
                ConnectionTestEventStatus::Success,
                locale_text(locale, "Gateway 鉴权通过", "Gateway authentication passed")
                    .to_string(),
            );
            Ok(ConnectionTestResult {
                success: true,
                step: ConnectionTestStep::GatewayProbe,
                summary: locale_text(
                    locale,
                    "SSH 隧道已建立，Gateway 鉴权通过。",
                    "SSH tunnel established and Gateway authentication passed.",
                )
                .into(),
                stdout: gateway_result.stdout,
                stderr: gateway_result.stderr,
            })
        } else {
            let error_message =
                gateway_probe_error_message(locale, gateway_probe.reason.as_deref());
            emit_connection_test_event(
                app,
                ConnectionTestStep::GatewayProbe,
                ConnectionTestEventStatus::Error,
                error_message,
            );
            Ok(gateway_probe_failure_result_with_reason(
                locale,
                gateway_result.stdout,
                gateway_result.stderr,
                gateway_probe.reason,
            ))
        }
    })();

    let _ = kill_child(&mut ssh_child, locale);

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
                summary: locale_text(
                    locale,
                    "SSH 隧道建立失败，请检查 SSH 登录信息和端口转发配置。",
                    "Failed to create the SSH tunnel. Check the SSH credentials and port forwarding settings.",
                )
                .into(),
                stdout: String::new(),
                stderr: error.to_string(),
            })
        }
    }
}

fn gateway_probe_failure_result(
    locale: LocalePreference,
    stdout: String,
    stderr: String,
) -> ConnectionTestResult {
    gateway_probe_failure_result_with_reason(locale, stdout, stderr, None)
}

fn gateway_probe_failure_result_with_reason(
    locale: LocalePreference,
    stdout: String,
    stderr: String,
    reason: Option<String>,
) -> ConnectionTestResult {
    ConnectionTestResult {
        success: false,
        step: ConnectionTestStep::GatewayProbe,
        summary: gateway_probe_error_message(locale, reason.as_deref()),
        stdout,
        stderr,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GatewayProbeEvaluation {
    success: bool,
    reason: Option<String>,
}

fn evaluate_gateway_probe(result: &CapturedOutput) -> GatewayProbeEvaluation {
    if !result.success {
        return GatewayProbeEvaluation {
            success: false,
            reason: detect_gateway_probe_reason(&result.stdout, &result.stderr),
        };
    }

    let Some(parsed) = parse_gateway_status_output(&result.stdout) else {
        return GatewayProbeEvaluation {
            success: true,
            reason: None,
        };
    };

    let auth_ok = parsed
        .get("auth")
        .and_then(|auth| auth.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let rpc_ok = parsed
        .get("rpc")
        .and_then(|rpc| rpc.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if auth_ok && rpc_ok {
        GatewayProbeEvaluation {
            success: true,
            reason: None,
        }
    } else {
        let reason = parsed
            .get("rpc")
            .and_then(|rpc| rpc.get("error"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                parsed
                    .get("auth")
                    .and_then(|auth| auth.get("error"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .or_else(|| detect_gateway_probe_reason(&result.stdout, &result.stderr));

        GatewayProbeEvaluation {
            success: false,
            reason,
        }
    }
}

fn parse_gateway_status_output(stdout: &str) -> Option<Value> {
    serde_json::from_str(stdout).ok()
}

fn detect_gateway_probe_reason(stdout: &str, stderr: &str) -> Option<String> {
    let combined = [stdout, stderr]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed = combined.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn gateway_probe_error_message(locale: LocalePreference, reason: Option<&str>) -> String {
    let reason = reason.unwrap_or_default().to_ascii_lowercase();
    if reason.contains("pairing required") {
        return locale_owned(
            locale,
            "Gateway 需要先完成 pairing。当前 Token 和 SSH 隧道已经通了，但远端 Gateway 还没有完成配对，请先在 Gateway 侧完成 pairing 后再重试。",
            "The Gateway requires pairing first. The token and SSH tunnel are working, but the remote Gateway has not been paired yet. Complete pairing on the Gateway side and try again.",
        );
    }

    locale_owned(
        locale,
        "Gateway 鉴权失败，请检查 Token、SSH 转发和远程 Gateway 状态。",
        "Gateway authentication failed. Check the token, SSH forwarding, and remote Gateway status.",
    )
}

fn build_runtime_ssh_command(
    locale: LocalePreference,
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    company_profile: &CompanyProfile,
    ssh_password: Option<&str>,
) -> Result<CommandSpec> {
    match runtime_target {
        RuntimeTarget::MacNative | RuntimeTarget::WindowsNative => {
            if let Some(password) = ssh_password {
                let askpass_program = std::env::current_exe().context(locale_text(
                    locale,
                    "无法定位 SSH 密码辅助程序",
                    "Failed to locate the SSH password helper",
                ))?;
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
        RuntimeTarget::MacNative | RuntimeTarget::WindowsNative => {
            build_native_gateway_status_command(company_profile, token, timeout_ms)
        }
        RuntimeTarget::WindowsWsl => {
            build_wsl_gateway_status_command(target_profile, company_profile, token, timeout_ms)
        }
    }
}

fn spawn_command_spec(spec: &CommandSpec, locale: LocalePreference) -> Result<Child> {
    let mut command = new_command(&spec.program, &spec.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in &spec.envs {
        command.env(key, value);
    }
    command.spawn().with_context(|| {
        format!(
            "{}: {}",
            locale_text(locale, "启动命令失败", "Failed to start command"),
            spec.program
        )
    })
}

fn run_command_capture(spec: &CommandSpec, locale: LocalePreference) -> Result<CapturedOutput> {
    let output = new_command(&spec.program, &spec.args)
        .envs(spec.envs.iter().cloned())
        .stdin(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "{}: {}",
                locale_text(locale, "执行命令失败", "Failed to run command"),
                spec.program
            )
        })?;

    Ok(CapturedOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn ensure_child_alive(child: &mut Child, locale: LocalePreference, label: &str) -> Result<()> {
    if let Some(status) = child.try_wait()? {
        return Err(anyhow!(
            "{}",
            locale_owned(
                locale,
                format!("{label} 提前退出，状态码: {status}"),
                format!("{label} exited early with status {status}")
            )
        ));
    }
    Ok(())
}

fn kill_child(child: &mut Child, locale: LocalePreference) -> Result<()> {
    child.kill().context(locale_text(
        locale,
        "结束临时连接进程失败",
        "Failed to stop the temporary connection process",
    ))?;
    let _ = child.wait();
    Ok(())
}

fn wait_for_local_port_ready(
    locale: LocalePreference,
    port: u16,
    max_wait: Duration,
    poll_interval: Duration,
) -> Result<()> {
    let started = std::time::Instant::now();
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    loop {
        match TcpStream::connect_timeout(&address, poll_interval) {
            Ok(_) => return Ok(()),
            Err(error) if started.elapsed() >= max_wait => {
                return Err(anyhow!(
                    "{}",
                    locale_owned(
                        locale,
                        format!("本地 SSH 隧道端口 {} 未就绪: {}", port, error),
                        format!("Local SSH tunnel port {} is not ready: {}", port, error)
                    )
                ));
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

fn emit_environment_snapshot(app: &AppHandle, snapshot: &EnvironmentSnapshot) {
    let _ = app.emit("environment-snapshot", snapshot.clone());
}

struct CapturedOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn install_recommendation(
    locale: LocalePreference,
    runtime_target: RuntimeTarget,
    host_ssh_installed: bool,
    host_openclaw_installed: bool,
    target_ssh_installed: bool,
    wsl_openclaw_installed: bool,
    wsl_status: Option<&WslStatus>,
    windows_discovery: Option<&WindowsDiscovery>,
) -> String {
    match runtime_target {
        RuntimeTarget::MacNative => {
            if !host_ssh_installed {
                if matches!(locale, LocalePreference::EnUs) {
                    "macOS built-in ssh was not detected. Fix system OpenSSH before continuing."
                        .into()
                } else {
                    "当前未检测到 macOS 自带的 ssh 命令，请先修复系统 OpenSSH 后再继续。".into()
                }
            } else {
                if matches!(locale, LocalePreference::EnUs) {
                    "Prefer the official install script first, then fall back to a global npm / pnpm install if needed.".into()
                } else {
                    "推荐先使用官方安装脚本，失败后再回退到 npm / pnpm 全局安装。".into()
                }
            }
        }
        RuntimeTarget::WindowsNative => {
            if windows_discovery
                .map(|discovery| matches!(discovery.phase, WindowsDiscoveryPhase::Pending))
                .unwrap_or(false)
            {
                return if matches!(locale, LocalePreference::EnUs) {
                    "BizClaw is detecting local Windows and WSL OpenClaw environments in the background. Local Windows installation stays the default install target.".into()
                } else {
                    "BizClaw 正在后台检测 Windows 本机与 WSL 环境，默认仍优先安装到 Windows 本机。"
                        .into()
                };
            }

            if host_openclaw_installed {
                if matches!(locale, LocalePreference::EnUs) {
                    "OpenClaw is already installed on local Windows. BizClaw will keep using the native Windows installation first.".into()
                } else {
                    "已检测到 Windows 本机 OpenClaw，BizClaw 会继续优先使用本机安装。".into()
                }
            } else if !target_ssh_installed {
                if matches!(locale, LocalePreference::EnUs) {
                    "BizClaw installs to local Windows first. It will prepare OpenSSH, Node.js, Git, and then run the OpenClaw installer.".into()
                } else {
                    "BizClaw 默认先安装到 Windows 本机，并按顺序补齐 OpenSSH、Node.js、Git 后再安装 OpenClaw。".into()
                }
            } else if matches!(locale, LocalePreference::EnUs) {
                "BizClaw will use the local Windows installation first. If you need a separate target, WSL remains available as an explicit install option.".into()
            } else {
                "BizClaw 将优先使用 Windows 本机安装；如果你需要单独的环境，也可以显式选择安装到 WSL。".into()
            }
        }
        RuntimeTarget::WindowsWsl => {
            if !host_openclaw_installed && !wsl_openclaw_installed {
                return if matches!(locale, LocalePreference::EnUs) {
                    "OpenClaw is not installed on Windows or WSL yet. BizClaw now installs to local Windows first, while WSL remains an explicit secondary install target.".into()
                } else {
                    "当前尚未在 Windows 本机或 WSL 中检测到 OpenClaw。BizClaw 默认先安装到 Windows 本机，同时保留显式的 WSL 安装入口。".into()
                };
            }
            if let Some(status) = wsl_status {
                if !status.ready {
                    return if matches!(locale, LocalePreference::EnUs) {
                        format!(
                            "Windows will run OpenClaw through WSL. {} is not ready yet, and BizClaw can bootstrap it automatically.",
                            status.distro_name
                        )
                    } else {
                        format!(
                            "Windows 将通过 WSL 运行 OpenClaw；当前 {} 尚未就绪，可由 BizClaw 自动引导安装。",
                            status.distro_name
                        )
                    };
                }
            }
            if !target_ssh_installed {
                if matches!(locale, LocalePreference::EnUs) {
                    "WSL is available, but Ubuntu is missing OpenSSH. BizClaw will install it during setup.".into()
                } else {
                    "已检测到 WSL，但 Ubuntu 中缺少 OpenSSH，BizClaw 会在安装流程里补齐。".into()
                }
            } else {
                if matches!(locale, LocalePreference::EnUs) {
                    "The Windows build always hosts OpenClaw Node and the SSH tunnel inside WSL Ubuntu.".into()
                } else {
                    "Windows 版本固定通过 WSL Ubuntu 托管运行 OpenClaw Node 与 SSH 隧道。".into()
                }
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
    let latest_version_cache = state.latest_openclaw_version_cache.clone();

    thread::spawn(move || {
        let locale = current_locale(&app_handle);
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
                    locale,
                    kind,
                    OperationStep::Detect,
                    "task-error",
                    &error.to_string(),
                ),
            ),
        };

        emit_operation_status(&app_handle, &task_snapshot);

        if let Ok(snapshot) =
            detect_environment_inner(&app_handle, &runtime_state, false, &latest_version_cache)
        {
            {
                let mut guard = environment_cache
                    .lock()
                    .expect("environment cache mutex poisoned");
                *guard = Some(snapshot.clone());
            }
            emit_environment_snapshot(&app_handle, &snapshot);
            let _ = app_menu::refresh_status_menu_from_state(&app_handle);
        } else {
            let _ = app_menu::refresh_status_menu_from_state(&app_handle);
        }
    });

    Ok(snapshot)
}

fn start_check_update_task(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<OperationTaskSnapshot> {
    let snapshot = start_task(
        &state.operation,
        OperationKind::CheckUpdate,
        OperationStep::CheckUpdate,
    )?;
    emit_operation_status(app, &snapshot);

    let app_handle = app.clone();
    let runtime_state = state.runtime.clone();
    let operation_state = state.operation.clone();
    let environment_cache = state.environment_cache.clone();
    let latest_version_cache = state.latest_openclaw_version_cache.clone();

    thread::spawn(move || {
        let locale = current_locale(&app_handle);
        emit_operation_event(
            &app_handle,
            Some(&operation_state),
            OperationKind::CheckUpdate,
            OperationStep::CheckUpdate,
            OperationEventStatus::Running,
            OperationEventSource::System,
            locale_text(
                locale,
                "正在检查 OpenClaw 更新",
                "Checking for OpenClaw updates",
            ),
        );

        let task_snapshot = match detect_environment_inner(
            &app_handle,
            &runtime_state,
            true,
            &latest_version_cache,
        ) {
            Ok(snapshot) => {
                {
                    let mut guard = environment_cache
                        .lock()
                        .expect("environment cache mutex poisoned");
                    *guard = Some(snapshot.clone());
                }
                emit_environment_snapshot(&app_handle, &snapshot);
                let message = if snapshot.update_available {
                    locale_owned(
                        locale,
                        format!(
                            "检测到新版本：{}",
                            snapshot.latest_openclaw_version.clone().unwrap_or_else(|| {
                                locale_text(locale, "未知版本", "Unknown version").into()
                            })
                        ),
                        format!(
                            "New version available: {}",
                            snapshot.latest_openclaw_version.clone().unwrap_or_else(|| {
                                locale_text(locale, "未知版本", "Unknown version").into()
                            })
                        ),
                    )
                } else {
                    locale_text(
                        locale,
                        "当前已是最新版本，或暂时无法获取远端版本信息。",
                        "OpenClaw is already up to date, or the remote version could not be loaded.",
                    )
                    .into()
                };

                let (phase, status, result, event_message) = if cancel_requested(&operation_state) {
                    (
                        OperationTaskPhase::Cancelled,
                        OperationEventStatus::Cancelled,
                        cancelled_result(
                            locale,
                            OperationKind::CheckUpdate,
                            OperationStep::CheckUpdate,
                            "cancelled",
                            String::new(),
                            String::new(),
                        ),
                        cancelled_follow_up(locale, OperationKind::CheckUpdate).into(),
                    )
                } else {
                    (
                        OperationTaskPhase::Success,
                        OperationEventStatus::Success,
                        OperationResult {
                            kind: OperationKind::CheckUpdate,
                            strategy: "background-check".into(),
                            success: true,
                            step: OperationStep::CheckUpdate,
                            stdout: String::new(),
                            stderr: String::new(),
                            needs_elevation: false,
                            manual_url: MANUAL_INSTALL_URL.into(),
                            follow_up: message.clone(),
                            remediation: None,
                        },
                        message.clone(),
                    )
                };

                emit_operation_event(
                    &app_handle,
                    Some(&operation_state),
                    OperationKind::CheckUpdate,
                    OperationStep::CheckUpdate,
                    status,
                    OperationEventSource::System,
                    event_message,
                );
                finish_task(&operation_state, phase, result)
            }
            Err(error) => {
                let message = error.to_string();
                emit_operation_event(
                    &app_handle,
                    Some(&operation_state),
                    OperationKind::CheckUpdate,
                    OperationStep::CheckUpdate,
                    OperationEventStatus::Error,
                    OperationEventSource::System,
                    message.clone(),
                );
                finish_task(
                    &operation_state,
                    if cancel_requested(&operation_state) {
                        OperationTaskPhase::Cancelled
                    } else {
                        OperationTaskPhase::Error
                    },
                    if cancel_requested(&operation_state) {
                        cancelled_result(
                            locale,
                            OperationKind::CheckUpdate,
                            OperationStep::CheckUpdate,
                            "cancelled",
                            String::new(),
                            String::new(),
                        )
                    } else {
                        failure_result(
                            locale,
                            OperationKind::CheckUpdate,
                            OperationStep::CheckUpdate,
                            "check-update-error",
                            &message,
                        )
                    },
                )
            }
        };

        emit_operation_status(&app_handle, &task_snapshot);
        let _ = app_menu::refresh_status_menu_from_state(&app_handle);
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
    let locale = current_locale(app);
    let latest_version_cache = app
        .state::<AppState>()
        .latest_openclaw_version_cache
        .clone();
    let snapshot = update_step(operation_state, OperationStep::Detect);
    emit_operation_status(app, &snapshot);
    emit_operation_event(
        app,
        Some(operation_state),
        kind.clone(),
        OperationStep::Detect,
        OperationEventStatus::Running,
        OperationEventSource::System,
        locale_text(
            locale,
            "正在检测目标运行环境",
            "Detecting the target runtime environment",
        ),
    );
    let mut resolved = resolve_environment(
        runtime_target.clone(),
        &target_profile,
        false,
        &latest_version_cache,
    );
    emit_operation_event(
        app,
        Some(operation_state),
        kind.clone(),
        OperationStep::Detect,
        OperationEventStatus::Success,
        OperationEventSource::System,
        locale_text(locale, "环境检测完成", "Environment detection completed"),
    );

    if let Some(result) =
        cancelled_result_if_requested(locale, operation_state, kind, OperationStep::Detect)
    {
        return Ok(result);
    }

    if matches!(kind, OperationKind::Install)
        && should_skip_install_for_target(&resolved, runtime_target)
    {
        return Ok(OperationResult {
            kind,
            strategy: "skipped".into(),
            success: true,
            step: OperationStep::InstallOpenClaw,
            stdout: String::new(),
            stderr: String::new(),
            needs_elevation: false,
            manual_url: MANUAL_INSTALL_URL.into(),
            follow_up: locale_text(
                locale,
                match runtime_target {
                    RuntimeTarget::WindowsNative => "已检测到 Windows 本机 OpenClaw，跳过安装。",
                    RuntimeTarget::WindowsWsl => "已检测到 WSL 中的 OpenClaw，跳过安装。",
                    RuntimeTarget::MacNative => "已检测到 OpenClaw，跳过安装。",
                },
                match runtime_target {
                    RuntimeTarget::WindowsNative => {
                        "A local Windows OpenClaw installation is already available. Skipping install."
                    }
                    RuntimeTarget::WindowsWsl => {
                        "An OpenClaw installation is already available in WSL. Skipping install."
                    }
                    RuntimeTarget::MacNative => "OpenClaw is already detected. Skipping installation.",
                },
            )
            .into(),
            remediation: None,
        });
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
                locale,
                kind.clone(),
                OperationStep::BootstrapWsl,
                current_platform()?,
                request.allow_elevation,
                &wsl_bootstrap_plan(&target_profile),
            )?;
            if cancel_requested(operation_state) {
                return Ok(cancelled_result(
                    locale,
                    kind,
                    OperationStep::BootstrapWsl,
                    bootstrap.strategy,
                    bootstrap.stdout,
                    bootstrap.stderr,
                ));
            }
            let ready_after = detect_wsl_status(&target_profile);
            if !bootstrap.success || !ready_after.ready {
                let remediation = remediation_for_failed_plan(request.allow_elevation, &bootstrap);
                let follow_up = if bootstrap.stdout.to_lowercase().contains("restart")
                    || bootstrap.stderr.to_lowercase().contains("restart")
                {
                    locale_text(
                        locale,
                        "WSL 安装已启动，请在 Windows 重启并完成 Ubuntu 初始化后重新打开 BizClaw。",
                        "WSL installation has started. Restart Windows, finish Ubuntu initialization, and reopen BizClaw.",
                    )
                } else {
                    locale_text(
                        locale,
                        "请完成 WSL / Ubuntu 初始化后，再回到 BizClaw 继续安装。",
                        "Finish WSL / Ubuntu initialization, then come back to BizClaw to continue the installation.",
                    )
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
                    follow_up: follow_up.to_string(),
                    remediation,
                });
            }
            resolved = resolve_environment(
                runtime_target.clone(),
                &target_profile,
                false,
                &latest_version_cache,
            );
        }
    }

    if !resolved.target_ssh_installed {
        let snapshot = update_step(operation_state, OperationStep::EnsureSsh);
        emit_operation_status(app, &snapshot);
        if matches!(runtime_target, RuntimeTarget::MacNative) && !command_available("brew") {
            return Ok(OperationResult {
                kind,
                strategy: "missing-homebrew".into(),
                success: false,
                step: OperationStep::EnsureSsh,
                stdout: String::new(),
                stderr: String::new(),
                needs_elevation: false,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: locale_text(
                    locale,
                    "当前未检测到 Homebrew。请先安装 Homebrew，再让 BizClaw 自动补齐 OpenSSH。",
                    "Homebrew is not installed. Install Homebrew first, then let BizClaw install OpenSSH automatically.",
                )
                .into(),
                remediation: Some(homebrew_install_remediation()),
            });
        }
        let ssh_plan = match runtime_target {
            RuntimeTarget::MacNative => macos_ensure_ssh_plan(),
            RuntimeTarget::WindowsNative => windows_native_ensure_ssh_plan(),
            RuntimeTarget::WindowsWsl => wsl_ensure_ssh_plan(&target_profile),
        };
        let ssh_result = run_single_plan(
            app,
            operation_state,
            locale,
            kind.clone(),
            OperationStep::EnsureSsh,
            current_platform()?,
            request.allow_elevation,
            &ssh_plan,
        )?;
        if cancel_requested(operation_state) {
            return Ok(cancelled_result(
                locale,
                kind,
                OperationStep::EnsureSsh,
                ssh_result.strategy,
                ssh_result.stdout,
                ssh_result.stderr,
            ));
        }
        if !ssh_result.success {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &ssh_result);
            return Ok(OperationResult {
                kind,
                strategy: ssh_result.strategy,
                success: false,
                step: OperationStep::EnsureSsh,
                stdout: ssh_result.stdout,
                stderr: ssh_result.stderr,
                needs_elevation: ssh_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: match runtime_target {
                    RuntimeTarget::MacNative => locale_text(
                        locale,
                        "BizClaw 未能通过 Homebrew 自动安装 OpenSSH，请查看输出后重试。",
                        "BizClaw could not install OpenSSH via Homebrew. Check the output and try again.",
                    ),
                    RuntimeTarget::WindowsNative => locale_text(
                        locale,
                        "BizClaw 未能自动安装 Windows 本机 OpenSSH，请查看输出后重试，并在系统授权弹窗中允许安装。",
                        "BizClaw could not install local Windows OpenSSH automatically. Check the output, then retry and allow the Windows elevation prompt to install it.",
                    ),
                    RuntimeTarget::WindowsWsl => locale_text(
                        locale,
                        "请确认 Ubuntu 已完成初始化，且当前用户具备安装 openssh-client 的 sudo 权限。",
                        "Make sure Ubuntu initialization is complete and the current user has sudo permission to install openssh-client.",
                    ),
                }
                .into(),
                remediation,
            });
        }
        if matches!(runtime_target, RuntimeTarget::WindowsNative) {
            let verification =
                verify_windows_ssh_installation(WINDOWS_INSTALL_VERIFICATION_TIMEOUT);
            if !matches!(verification, WindowsInstallVerification::Verified { .. }) {
                let remediation = remediation_for_failed_plan(request.allow_elevation, &ssh_result);
                return Ok(OperationResult {
                    kind,
                    strategy: ssh_result.strategy,
                    success: false,
                    step: OperationStep::EnsureSsh,
                    stdout: ssh_result.stdout,
                    stderr: ssh_result.stderr,
                    needs_elevation: ssh_result.needs_elevation,
                    manual_url: MANUAL_INSTALL_URL.into(),
                    follow_up: openssh_verification_follow_up(locale, &verification),
                    remediation,
                });
            }
        }
        resolved = resolve_environment(
            runtime_target.clone(),
            &target_profile,
            false,
            &latest_version_cache,
        );
        if !resolved.target_ssh_installed {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &ssh_result);
            return Ok(OperationResult {
                kind,
                strategy: ssh_result.strategy,
                success: false,
                step: OperationStep::EnsureSsh,
                stdout: ssh_result.stdout,
                stderr: ssh_result.stderr,
                needs_elevation: ssh_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: match runtime_target {
                    RuntimeTarget::MacNative => locale_text(
                        locale,
                        "OpenSSH 安装步骤已执行，但当前仍未检测到 ssh 命令，请检查输出后重试。",
                        "The OpenSSH install step completed, but the ssh command is still unavailable. Check the output and try again.",
                    ),
                    RuntimeTarget::WindowsNative => locale_text(
                        locale,
                        "OpenSSH 安装步骤已执行，但当前仍未检测到 ssh 命令。请检查系统 OpenSSH Client 是否安装成功，或重新打开终端后再试。",
                        "The OpenSSH install step completed, but the ssh command is still unavailable. Check whether Windows OpenSSH Client finished installing, or reopen the terminal and try again.",
                    ),
                    RuntimeTarget::WindowsWsl => locale_text(
                        locale,
                        "OpenSSH 安装步骤已执行，但 Ubuntu 中仍未检测到 ssh 命令，请检查输出后重试。",
                        "The OpenSSH install step completed, but ssh is still unavailable inside Ubuntu. Check the output and try again.",
                    ),
                }
                .into(),
                remediation,
            });
        }
    }

    if matches!(runtime_target, RuntimeTarget::WindowsNative) && !windows_local_node_ready() {
        let snapshot = update_step(operation_state, OperationStep::EnsureNode);
        emit_operation_status(app, &snapshot);

        let Some(node_plan) = windows_native_ensure_node_plan() else {
            return Ok(OperationResult {
                kind,
                strategy: "missing-node-msi".into(),
                success: false,
                step: OperationStep::EnsureNode,
                stdout: String::new(),
                stderr: String::new(),
                needs_elevation: false,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: locale_text(
                    locale,
                    "BizClaw 未能准备 Node.js 官方安装器，请检查网络连接后重试。",
                    "BizClaw could not prepare the official Node.js installer. Check the network connection and try again.",
                )
                .into(),
                remediation: None,
            });
        };

        let node_result = run_single_plan(
            app,
            operation_state,
            locale,
            kind.clone(),
            OperationStep::EnsureNode,
            current_platform()?,
            request.allow_elevation,
            &node_plan,
        )?;
        if cancel_requested(operation_state) {
            return Ok(cancelled_result(
                locale,
                kind,
                OperationStep::EnsureNode,
                node_result.strategy,
                node_result.stdout,
                node_result.stderr,
            ));
        }
        if !node_result.success {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &node_result);
            let follow_up = node_install_failure_follow_up(locale, &node_result);
            return Ok(OperationResult {
                kind,
                strategy: node_result.strategy,
                success: false,
                step: OperationStep::EnsureNode,
                stdout: node_result.stdout,
                stderr: node_result.stderr,
                needs_elevation: node_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up,
                remediation,
            });
        }
        let node_verification =
            verify_windows_node_installation(WINDOWS_INSTALL_VERIFICATION_TIMEOUT);
        if !matches!(
            node_verification,
            WindowsInstallVerification::Verified { .. }
        ) {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &node_result);
            return Ok(OperationResult {
                kind,
                strategy: node_result.strategy,
                success: false,
                step: OperationStep::EnsureNode,
                stdout: node_result.stdout,
                stderr: node_result.stderr,
                needs_elevation: node_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: node_verification_follow_up(locale, &node_verification),
                remediation,
            });
        }
        if !node_result.success {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &node_result);
            return Ok(OperationResult {
                kind,
                strategy: node_result.strategy,
                success: false,
                step: OperationStep::EnsureNode,
                stdout: node_result.stdout,
                stderr: node_result.stderr,
                needs_elevation: node_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: locale_text(
                    locale,
                    "BizClaw 未能自动安装 Node.js，请检查输出后重试，并在系统授权弹窗中允许安装。",
                    "BizClaw could not install Node.js automatically. Check the output, then retry and allow the Windows elevation prompt to continue.",
                )
                .into(),
                remediation,
            });
        }
        if !windows_local_node_ready() {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &node_result);
            let detected_version = windows_local_node_version();
            return Ok(OperationResult {
                kind,
                strategy: node_result.strategy,
                success: false,
                step: OperationStep::EnsureNode,
                stdout: node_result.stdout,
                stderr: node_result.stderr,
                needs_elevation: node_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: locale_owned(
                    locale,
                    match detected_version.as_ref() {
                        Some(version) => format!(
                            "Node.js 安装步骤已执行，但当前版本 {} 仍低于要求的 {}+。请检查输出后重试。",
                            version, WINDOWS_NODE_MIN_MAJOR
                        ),
                        None => "Node.js 安装步骤已执行，但当前仍未检测到 node 命令。请重新打开终端后重试。".into(),
                    },
                    match detected_version.as_ref() {
                        Some(version) => format!(
                            "The Node.js install step completed, but the detected version {} is still below the required {}+. Check the output and try again.",
                            version, WINDOWS_NODE_MIN_MAJOR
                        ),
                        None => "The Node.js install step completed, but the node command is still unavailable. Reopen the terminal and try again.".into(),
                    },
                ),
                remediation,
            });
        }
    }

    if matches!(runtime_target, RuntimeTarget::WindowsNative) && !windows_local_git_ready() {
        let snapshot = update_step(operation_state, OperationStep::EnsureGit);
        emit_operation_status(app, &snapshot);

        let Some(git_plan) = windows_native_ensure_git_plan() else {
            return Ok(OperationResult {
                kind,
                strategy: "missing-git-installer".into(),
                success: false,
                step: OperationStep::EnsureGit,
                stdout: String::new(),
                stderr: String::new(),
                needs_elevation: false,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: locale_text(
                    locale,
                    "BizClaw 未能准备 Git 官方安装器，请检查网络连接后重试。",
                    "BizClaw could not prepare the official Git installer. Check the network connection and try again.",
                )
                .into(),
                remediation: None,
            });
        };

        let git_result = run_single_plan(
            app,
            operation_state,
            locale,
            kind.clone(),
            OperationStep::EnsureGit,
            current_platform()?,
            request.allow_elevation,
            &git_plan,
        )?;
        if cancel_requested(operation_state) {
            return Ok(cancelled_result(
                locale,
                kind,
                OperationStep::EnsureGit,
                git_result.strategy,
                git_result.stdout,
                git_result.stderr,
            ));
        }
        if !git_result.success {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &git_result);
            let follow_up = git_install_failure_follow_up(locale, &git_result);
            return Ok(OperationResult {
                kind,
                strategy: git_result.strategy,
                success: false,
                step: OperationStep::EnsureGit,
                stdout: git_result.stdout,
                stderr: git_result.stderr,
                needs_elevation: git_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up,
                remediation,
            });
        }
        let git_verification =
            verify_windows_git_installation(WINDOWS_INSTALL_VERIFICATION_TIMEOUT);
        if !matches!(
            git_verification,
            WindowsInstallVerification::Verified { .. }
        ) {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &git_result);
            return Ok(OperationResult {
                kind,
                strategy: git_result.strategy,
                success: false,
                step: OperationStep::EnsureGit,
                stdout: git_result.stdout,
                stderr: git_result.stderr,
                needs_elevation: git_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: git_verification_follow_up(locale, &git_verification),
                remediation,
            });
        }
        if !git_result.success {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &git_result);
            return Ok(OperationResult {
                kind,
                strategy: git_result.strategy,
                success: false,
                step: OperationStep::EnsureGit,
                stdout: git_result.stdout,
                stderr: git_result.stderr,
                needs_elevation: git_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: locale_text(
                    locale,
                    "BizClaw 未能自动安装 Git。请查看输出后重试，并在系统授权弹窗中允许安装。",
                    "BizClaw could not install Git automatically. Check the output, then retry and allow the Windows elevation prompt to continue.",
                )
                .into(),
                remediation,
            });
        }
        if !windows_local_git_ready() {
            let remediation = remediation_for_failed_plan(request.allow_elevation, &git_result);
            let detected_version = windows_local_git_version();
            return Ok(OperationResult {
                kind,
                strategy: git_result.strategy,
                success: false,
                step: OperationStep::EnsureGit,
                stdout: git_result.stdout,
                stderr: git_result.stderr,
                needs_elevation: git_result.needs_elevation,
                manual_url: MANUAL_INSTALL_URL.into(),
                follow_up: locale_owned(
                    locale,
                    match detected_version.as_ref() {
                        Some(version) => format!(
                            "Git 安装步骤已执行，但当前检测到的版本仍异常：{}。请检查输出后重试。",
                            version
                        ),
                        None => "Git 安装步骤已执行，但当前仍未检测到 git 命令。请重新打开终端后重试。".into(),
                    },
                    match detected_version.as_ref() {
                        Some(version) => format!(
                            "The Git install step completed, but the detected version still looks wrong: {}. Check the output and try again.",
                            version
                        ),
                        None => "The Git install step completed, but the git command is still unavailable. Reopen the terminal and try again.".into(),
                    },
                ),
                remediation,
            });
        }
    }

    if matches!(kind, OperationKind::Update) {
        resolved = resolve_environment(
            runtime_target.clone(),
            &target_profile,
            true,
            &latest_version_cache,
        );
        let snapshot = update_step(operation_state, OperationStep::CheckUpdate);
        emit_operation_status(app, &snapshot);
        emit_operation_event(
            app,
            Some(operation_state),
            kind.clone(),
            OperationStep::CheckUpdate,
            OperationEventStatus::Running,
            OperationEventSource::System,
            locale_text(
                locale,
                "正在获取最新 OpenClaw 版本信息",
                "Loading the latest OpenClaw version information",
            ),
        );
        if !resolved.openclaw_installed {
            return Ok(failure_result(
                locale,
                kind,
                OperationStep::CheckUpdate,
                "missing-openclaw",
                locale_text(
                    locale,
                    "请先安装 OpenClaw，再执行更新。",
                    "Install OpenClaw before running an update.",
                ),
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
                locale_text(
                    locale,
                    "当前已是最新版本，无需更新",
                    "OpenClaw is already up to date",
                ),
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
                follow_up: locale_text(
                    locale,
                    "当前已是最新版本。",
                    "OpenClaw is already up to date.",
                )
                .into(),
                remediation: None,
            });
        }
        emit_operation_event(
            app,
            Some(operation_state),
            kind.clone(),
            OperationStep::CheckUpdate,
            OperationEventStatus::Success,
            OperationEventSource::System,
            locale_owned(
                locale,
                format!(
                    "发现新版本 {}",
                    resolved
                        .latest_openclaw_version
                        .clone()
                        .unwrap_or_else(
                            || locale_text(locale, "未知版本", "Unknown version").into()
                        )
                ),
                format!(
                    "New version found: {}",
                    resolved
                        .latest_openclaw_version
                        .clone()
                        .unwrap_or_else(
                            || locale_text(locale, "未知版本", "Unknown version").into()
                        )
                ),
            ),
        );
        if let Some(result) =
            cancelled_result_if_requested(locale, operation_state, kind, OperationStep::CheckUpdate)
        {
            return Ok(result);
        }
    } else if should_skip_install_for_target(&resolved, runtime_target) {
        return Ok(OperationResult {
            kind,
            strategy: "skipped".into(),
            success: true,
            step: OperationStep::InstallOpenClaw,
            stdout: String::new(),
            stderr: String::new(),
            needs_elevation: false,
            manual_url: MANUAL_INSTALL_URL.into(),
            follow_up: locale_text(
                locale,
                "已检测到 OpenClaw，跳过安装。",
                "OpenClaw is already detected. Skipping installation.",
            )
            .into(),
            remediation: None,
        });
    }

    let platform = current_platform()?;
    let has_npm = target_command_available(runtime_target.clone(), &target_profile, "npm");
    let has_pnpm = target_command_available(runtime_target.clone(), &target_profile, "pnpm");
    let plans = if matches!(kind, OperationKind::Update) {
        update_plans_for_target(
            runtime_target.clone(),
            &target_profile,
            request.prefer_official,
            has_npm,
            has_pnpm,
        )
    } else {
        install_plans_for_target(
            runtime_target.clone(),
            &target_profile,
            request.prefer_official,
            has_npm,
            has_pnpm,
        )
    };

    let step = if matches!(kind, OperationKind::Update) {
        OperationStep::UpdateOpenClaw
    } else {
        OperationStep::InstallOpenClaw
    };
    let snapshot = update_step(operation_state, step);
    emit_operation_status(app, &snapshot);

    let mut last_result = None;
    for plan in plans {
        if let Some(result) = cancelled_result_if_requested(locale, operation_state, kind, step) {
            return Ok(result);
        }

        let result = run_single_plan(
            app,
            operation_state,
            locale,
            kind.clone(),
            step.clone(),
            platform,
            request.allow_elevation,
            &plan,
        )?;
        let cancelled = cancel_requested(operation_state);
        let openclaw_available =
            target_command_available(runtime_target.clone(), &target_profile, "openclaw")
                || windows_native_install_verified(kind.clone(), runtime_target.clone())
                || windows_native_installer_output_indicates_success(
                    kind.clone(),
                    runtime_target.clone(),
                    &result,
                );
        if should_treat_plan_as_success(kind, cancelled, result.success, openclaw_available) {
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
                    locale_text(
                        locale,
                        "OpenClaw 更新完成，请重新检测版本或直接启动连接。",
                        "OpenClaw update completed. Refresh the version check or start the connection now.",
                    )
                    .into()
                } else {
                    locale_text(
                        locale,
                        "OpenClaw 安装完成，请继续保存连接并启动托管；如果当前终端还未识别新命令，请重新打开终端。若 PowerShell 里仍无法直接使用 openclaw，可先改用 openclaw.cmd。",
                        "OpenClaw installation completed. Save the connection settings and start the hosted runtime. If an existing terminal still does not see the new commands, reopen it. If PowerShell still cannot run openclaw directly, use openclaw.cmd first.",
                    )
                    .into()
                },
                remediation: None,
            });
        }
        if cancelled {
            return Ok(cancelled_result(
                locale,
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
        exit_code: -1,
        stdout: String::new(),
        stderr: locale_text(locale, "安装未执行。", "Installation did not run.").into(),
        needs_elevation: false,
    });
    let remediation = remediation_for_failed_plan(request.allow_elevation, &result);
    Ok(OperationResult {
        kind,
        strategy: result.strategy,
        success: false,
        step,
        stdout: result.stdout,
        stderr: result.stderr,
        needs_elevation: result.needs_elevation,
        manual_url: MANUAL_INSTALL_URL.into(),
        follow_up: locale_text(
            locale,
            "操作未完成，请查看输出后重试，或改用官方手动安装文档。",
            "The action did not complete. Check the output and try again, or use the official manual installation guide.",
        )
        .into(),
        remediation,
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

fn windows_native_install_verified(kind: OperationKind, runtime_target: RuntimeTarget) -> bool {
    matches!(kind, OperationKind::Install)
        && matches!(runtime_target, RuntimeTarget::WindowsNative)
        && matches!(
            verify_windows_openclaw_installation(WINDOWS_INSTALL_VERIFICATION_TIMEOUT),
            WindowsInstallVerification::Verified { .. }
        )
}

fn windows_native_installer_output_indicates_success(
    kind: OperationKind,
    runtime_target: RuntimeTarget,
    result: &StreamedPlanResult,
) -> bool {
    if !matches!(kind, OperationKind::Install)
        || !matches!(runtime_target, RuntimeTarget::WindowsNative)
    {
        return false;
    }

    [result.stdout.as_str(), result.stderr.as_str()]
        .into_iter()
        .any(installer_output_indicates_openclaw_success)
}

fn installer_output_indicates_openclaw_success(output: &str) -> bool {
    let normalized = output.to_ascii_lowercase();
    normalized.contains("openclaw installed successfully")
        || normalized.contains("[ok] openclaw installed")
}

fn should_skip_install_for_target(
    resolved: &ResolvedEnvironment,
    runtime_target: RuntimeTarget,
) -> bool {
    if !resolved.openclaw_installed {
        return false;
    }

    match runtime_target {
        RuntimeTarget::MacNative | RuntimeTarget::WindowsNative | RuntimeTarget::WindowsWsl => true,
    }
}

fn cancelled_follow_up(locale: LocalePreference, kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Install => locale_text(locale, "安装已停止。", "Installation stopped."),
        OperationKind::Update => locale_text(locale, "更新已停止。", "Update stopped."),
        OperationKind::CheckUpdate => {
            locale_text(locale, "检查更新已停止。", "Update check stopped.")
        }
    }
}

fn cancelled_result_if_requested(
    locale: LocalePreference,
    operation_state: &SharedOperationState,
    kind: OperationKind,
    step: OperationStep,
) -> Option<OperationResult> {
    cancel_requested(operation_state).then(|| {
        cancelled_result(
            locale,
            kind,
            step,
            "cancelled",
            String::new(),
            String::new(),
        )
    })
}

fn cancelled_result(
    locale: LocalePreference,
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
        follow_up: cancelled_follow_up(locale, kind).into(),
        remediation: None,
    }
}

fn failure_result(
    _locale: LocalePreference,
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
        remediation: None,
    }
}

fn request_elevation_remediation() -> OperationRemediation {
    OperationRemediation {
        kind: OperationRemediationKind::RequestElevation,
        url_target: None,
    }
}

fn homebrew_install_remediation() -> OperationRemediation {
    OperationRemediation {
        kind: OperationRemediationKind::InstallHomebrew,
        url_target: Some(SupportUrlTarget::HomebrewInstall),
    }
}

fn remediation_for_failed_plan(
    allow_elevation: bool,
    result: &StreamedPlanResult,
) -> Option<OperationRemediation> {
    (!allow_elevation && result.needs_elevation).then(request_elevation_remediation)
}

fn run_single_plan(
    app: &AppHandle,
    operation_state: &SharedOperationState,
    locale: LocalePreference,
    kind: OperationKind,
    step: OperationStep,
    platform: Platform,
    allow_elevation: bool,
    plan: &InstallPlan,
) -> Result<StreamedPlanResult> {
    let result = execute_plan_streaming(
        app,
        operation_state,
        locale,
        kind.clone(),
        step.clone(),
        plan,
    )?;
    if cancel_requested(operation_state)
        || result.success
        || !result.needs_elevation
        || !allow_elevation
    {
        return Ok(result);
    }

    if let Some(elevated_plan) = elevated_install_plan(plan, platform) {
        return execute_plan_streaming(app, operation_state, locale, kind, step, &elevated_plan);
    }

    Ok(result)
}

fn execute_plan_streaming(
    app: &AppHandle,
    operation_state: &SharedOperationState,
    locale: LocalePreference,
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
        locale_owned(
            locale,
            format!("开始执行 {}", plan.strategy),
            format!("Starting {}", plan.strategy),
        ),
    );

    let mut command = new_command(&plan.program, &plan.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in &plan.envs {
        command.env(key, value);
    }

    let mut child = command.spawn().with_context(|| {
        format!(
            "{}: {}",
            locale_text(locale, "执行命令失败", "Failed to run command"),
            plan.program
        )
    })?;
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
            let Some(message) = sanitize_operation_log_line(&message) else {
                continue;
            };
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
        let Some(message) = sanitize_operation_log_line(&message) else {
            continue;
        };
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

    let status = exit_status.expect("operation exit status should be available");
    let exit_code = status.code().unwrap_or(-1);
    let success = status.success() && !cancelled;
    let needs_elevation =
        should_retry_with_elevation(elevation_command_name(plan), exit_code, &stderr_buffer);
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
            locale_owned(
                locale,
                format!("{} 已停止", plan.strategy),
                format!("{} stopped", plan.strategy),
            )
        } else if success {
            locale_owned(
                locale,
                format!("{} 执行完成", plan.strategy),
                format!("{} completed", plan.strategy),
            )
        } else {
            locale_owned(
                locale,
                format!("{} 执行失败，退出状态 {}", plan.strategy, status),
                format!("{} failed with exit status {}", plan.strategy, status),
            )
        },
    );

    Ok(StreamedPlanResult {
        strategy: plan.strategy.into(),
        success,
        exit_code,
        stdout: stdout_buffer,
        stderr: stderr_buffer,
        needs_elevation,
    })
}

fn elevation_command_name(plan: &InstallPlan) -> &str {
    match plan.strategy {
        "ensure-node-download" => "node",
        "ensure-git-download" => "git",
        "ensure-ssh" | "ensure-ssh-download" => "ssh",
        _ => plan.program.as_str(),
    }
}

fn read_stream(
    reader: impl std::io::Read + Send + 'static,
    source: OperationEventSource,
    tx: mpsc::Sender<(OperationEventSource, String)>,
) {
    let reader = BufReader::new(reader);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let Some(sanitized) = sanitize_operation_log_line(&line) else {
            continue;
        };
        let _ = tx.send((source.clone(), sanitized));
    }
}

fn sanitize_operation_log_line(line: &str) -> Option<String> {
    let mut cleaned: String = line
        .chars()
        .filter(|ch| *ch == '\t' || !ch.is_control())
        .collect();
    cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        return None;
    }

    let without_spinner_prefix = cleaned
        .trim_start_matches(|ch: char| matches!(ch, '-' | '\\' | '|' | '/' | ' ' | '\t'))
        .trim()
        .to_string();
    let candidate = if without_spinner_prefix.is_empty() {
        cleaned
    } else {
        without_spinner_prefix
    };

    if candidate
        .chars()
        .all(|ch| matches!(ch, '-' | '\\' | '|' | '/' | ' '))
    {
        return None;
    }

    Some(candidate)
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
    use tauri::{window::Color, Theme};

    use super::{
        apply_ui_preferences_to_snapshot, build_environment_snapshot, cancelled_follow_up,
        evaluate_gateway_probe, gateway_probe_failure_result,
        gateway_probe_failure_result_with_reason, inspect_token_state,
        installer_output_indicates_openclaw_success, load_saved_token, persist_profile_atomic,
        should_skip_install_for_target, should_treat_plan_as_success,
        update_available_from_versions, window_background_color_for_theme,
        window_theme_for_preference, with_cached_latest_openclaw_version, CapturedOutput,
        ResolvedEnvironment, RuntimePhase, RuntimeStatus, SettingsStoreAccess, TokenState,
    };
    use crate::{
        secret_store::{MemorySecretStore, SecretStore},
        types::{
            CompanyProfile, ConnectionTestStep, LocalePreference, OperationKind, PersistedSettings,
            RuntimeTarget, TargetProfile, TokenStatus, UiPreferences, UserProfile,
            WindowsDiscovery, WindowsDiscoveryPhase, WindowsNativeDiscovery, WindowsWslDiscovery,
            WslStatus,
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
            UiPreferences::default(),
            RuntimeStatus::default(),
            ResolvedEnvironment {
                runtime_target: RuntimeTarget::MacNative,
                host_ssh_installed: true,
                host_openclaw_installed: true,
                target_ssh_installed: true,
                wsl_openclaw_installed: false,
                openclaw_installed: true,
                openclaw_version: Some("OpenClaw 2026.3.8".into()),
                latest_openclaw_version: Some("2026.3.9".into()),
                update_available: true,
                wsl_status: None,
                windows_discovery: None,
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
            UiPreferences::default(),
            RuntimeStatus::default(),
            ResolvedEnvironment {
                runtime_target: RuntimeTarget::WindowsWsl,
                host_ssh_installed: false,
                host_openclaw_installed: false,
                target_ssh_installed: false,
                wsl_openclaw_installed: false,
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
                windows_discovery: Some(WindowsDiscovery {
                    phase: WindowsDiscoveryPhase::Ready,
                    native: WindowsNativeDiscovery {
                        ssh_installed: false,
                        node_installed: false,
                        node_version: None,
                        git_installed: false,
                        git_version: None,
                        openclaw_installed: false,
                        openclaw_version: None,
                    },
                    wsl: WindowsWslDiscovery {
                        status: Some(WslStatus {
                            available: true,
                            distro_name: "Ubuntu".into(),
                            distro_installed: false,
                            ready: false,
                            needs_reboot: false,
                            message: Some("missing".into()),
                        }),
                        ssh_installed: false,
                        openclaw_installed: false,
                        openclaw_version: None,
                    },
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
        assert_eq!(
            snapshot
                .windows_discovery
                .as_ref()
                .map(|discovery| discovery.phase),
            Some(WindowsDiscoveryPhase::Ready)
        );
        assert_eq!(snapshot.runtime_status.phase, RuntimePhase::InstallNeeded);
    }

    #[test]
    fn environment_snapshot_omits_wsl_status_when_target_is_windows_native() {
        let snapshot = build_environment_snapshot(
            "windows",
            RuntimeTarget::WindowsNative,
            Some(sample_settings()),
            UiPreferences::default(),
            RuntimeStatus::default(),
            ResolvedEnvironment {
                runtime_target: RuntimeTarget::WindowsNative,
                host_ssh_installed: true,
                host_openclaw_installed: true,
                target_ssh_installed: true,
                wsl_openclaw_installed: false,
                openclaw_installed: true,
                openclaw_version: Some("OpenClaw 2026.3.8".into()),
                latest_openclaw_version: Some("2026.3.9".into()),
                update_available: true,
                wsl_status: None,
                windows_discovery: Some(WindowsDiscovery {
                    phase: WindowsDiscoveryPhase::Ready,
                    native: WindowsNativeDiscovery {
                        ssh_installed: true,
                        node_installed: true,
                        node_version: Some("v24.0.0".into()),
                        git_installed: true,
                        git_version: Some("git version 2.53.0.windows.1".into()),
                        openclaw_installed: true,
                        openclaw_version: Some("OpenClaw 2026.3.8".into()),
                    },
                    wsl: WindowsWslDiscovery {
                        status: None,
                        ssh_installed: false,
                        openclaw_installed: false,
                        openclaw_version: None,
                    },
                }),
            },
            TokenState {
                status: TokenStatus::Saved,
                message: None,
            },
        );

        assert_eq!(snapshot.runtime_target, RuntimeTarget::WindowsNative);
        assert_eq!(snapshot.wsl_status, None);
        assert_eq!(
            snapshot
                .windows_discovery
                .as_ref()
                .and_then(|discovery| discovery.native.node_version.as_deref()),
            Some("v24.0.0")
        );
        assert_eq!(snapshot.runtime_status.phase, RuntimePhase::Configured);
    }

    #[test]
    fn install_skip_checks_only_the_selected_target_openclaw_presence() {
        let native_resolved = ResolvedEnvironment {
            runtime_target: RuntimeTarget::WindowsNative,
            host_ssh_installed: false,
            host_openclaw_installed: true,
            target_ssh_installed: false,
            wsl_openclaw_installed: false,
            openclaw_installed: true,
            openclaw_version: Some("OpenClaw 2026.3.8".into()),
            latest_openclaw_version: None,
            update_available: false,
            wsl_status: None,
            windows_discovery: None,
        };
        let wsl_resolved = ResolvedEnvironment {
            runtime_target: RuntimeTarget::WindowsWsl,
            host_ssh_installed: true,
            host_openclaw_installed: false,
            target_ssh_installed: false,
            wsl_openclaw_installed: true,
            openclaw_installed: true,
            openclaw_version: Some("OpenClaw 2026.3.8".into()),
            latest_openclaw_version: None,
            update_available: false,
            wsl_status: Some(WslStatus {
                available: true,
                distro_name: "Ubuntu".into(),
                distro_installed: true,
                ready: true,
                needs_reboot: false,
                message: None,
            }),
            windows_discovery: None,
        };

        assert!(should_skip_install_for_target(
            &native_resolved,
            RuntimeTarget::WindowsNative
        ));
        assert!(should_skip_install_for_target(
            &wsl_resolved,
            RuntimeTarget::WindowsWsl
        ));
    }

    #[test]
    fn gateway_probe_failure_result_uses_gateway_step_and_summary() {
        let result = gateway_probe_failure_result(
            LocalePreference::ZhCn,
            String::new(),
            "执行命令失败： openclaw".into(),
        );

        assert!(!result.success);
        assert_eq!(result.step, ConnectionTestStep::GatewayProbe);
        assert_eq!(result.stderr, "执行命令失败： openclaw");
        assert!(result.summary.contains("Gateway 鉴权失败"));
    }

    #[test]
    fn gateway_probe_failure_result_surfaces_pairing_required_message() {
        let result = gateway_probe_failure_result_with_reason(
            LocalePreference::EnUs,
            String::new(),
            "gateway connect failed".into(),
            Some("pairing required".into()),
        );

        assert!(!result.success);
        assert!(result.summary.contains("requires pairing"));
    }

    #[test]
    fn evaluate_gateway_probe_treats_rpc_failure_as_connection_failure() {
        let evaluation = evaluate_gateway_probe(&CapturedOutput {
            success: true,
            stdout: r#"{"auth":{"ok":true},"rpc":{"ok":false,"error":"gateway closed (1008): pairing required"}}"#.into(),
            stderr: "gateway connect failed: GatewayClientRequestError: pairing required".into(),
        });

        assert!(!evaluation.success);
        assert_eq!(
            evaluation.reason.as_deref(),
            Some("gateway closed (1008): pairing required")
        );
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
            LocalePreference::ZhCn,
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
            LocalePreference::EnUs,
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
            LocalePreference::ZhCn,
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
        let error =
            load_saved_token(LocalePreference::ZhCn, &MemorySecretStore::default()).unwrap_err();

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
    fn recognizes_successful_windows_installer_output_even_if_path_check_lags() {
        assert!(installer_output_indicates_openclaw_success(
            "[OK] OpenClaw installed\nOpenClaw installed successfully (2026.3.13)!\nopenclaw command not found on PATH."
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
        assert_eq!(
            cancelled_follow_up(LocalePreference::ZhCn, OperationKind::Install),
            "安装已停止。"
        );
        assert_eq!(
            cancelled_follow_up(LocalePreference::EnUs, OperationKind::Update),
            "Update stopped."
        );
    }

    #[test]
    fn applying_ui_preferences_updates_snapshot_locale_and_recommendation() {
        let mut snapshot = build_environment_snapshot(
            "windows",
            RuntimeTarget::WindowsWsl,
            Some(sample_settings()),
            UiPreferences::default(),
            RuntimeStatus::default(),
            ResolvedEnvironment {
                runtime_target: RuntimeTarget::WindowsWsl,
                host_ssh_installed: true,
                host_openclaw_installed: false,
                target_ssh_installed: false,
                wsl_openclaw_installed: false,
                openclaw_installed: false,
                openclaw_version: None,
                latest_openclaw_version: None,
                update_available: false,
                wsl_status: Some(WslStatus {
                    available: true,
                    distro_name: "Ubuntu".into(),
                    distro_installed: true,
                    ready: true,
                    needs_reboot: false,
                    message: None,
                }),
                windows_discovery: Some(WindowsDiscovery {
                    phase: WindowsDiscoveryPhase::Ready,
                    native: WindowsNativeDiscovery {
                        ssh_installed: true,
                        node_installed: true,
                        node_version: Some("v24.0.0".into()),
                        git_installed: true,
                        git_version: Some("git version 2.53.0.windows.1".into()),
                        openclaw_installed: false,
                        openclaw_version: None,
                    },
                    wsl: WindowsWslDiscovery {
                        status: Some(WslStatus {
                            available: true,
                            distro_name: "Ubuntu".into(),
                            distro_installed: true,
                            ready: true,
                            needs_reboot: false,
                            message: None,
                        }),
                        ssh_installed: false,
                        openclaw_installed: false,
                        openclaw_version: None,
                    },
                }),
            },
            TokenState {
                status: TokenStatus::Missing,
                message: None,
            },
        );

        apply_ui_preferences_to_snapshot(
            &mut snapshot,
            UiPreferences {
                theme: crate::types::ThemePreference::Dark,
                locale: LocalePreference::EnUs,
                sidebar_collapsed: true,
            },
        );

        assert_eq!(snapshot.ui_preferences.locale, LocalePreference::EnUs);
        assert_eq!(
            snapshot.install_recommendation,
            "OpenClaw is not installed on Windows or WSL yet. BizClaw now installs to local Windows first, while WSL remains an explicit secondary install target."
        );
    }

    #[test]
    fn system_theme_maps_to_native_follow_system_behavior() {
        assert_eq!(
            window_theme_for_preference(crate::types::ThemePreference::Light),
            Some(Theme::Light)
        );
        assert_eq!(
            window_theme_for_preference(crate::types::ThemePreference::Dark),
            Some(Theme::Dark)
        );
        assert_eq!(
            window_theme_for_preference(crate::types::ThemePreference::System),
            None
        );
    }

    #[test]
    fn dark_theme_uses_deep_window_background_color() {
        assert_eq!(
            window_background_color_for_theme(crate::types::ThemePreference::Dark),
            Some(Color(11, 18, 32, 255))
        );
        assert_eq!(
            window_background_color_for_theme(crate::types::ThemePreference::Light),
            Some(Color(245, 247, 251, 255))
        );
        assert_eq!(
            window_background_color_for_theme(crate::types::ThemePreference::System),
            None
        );
    }

    #[test]
    fn latest_version_cache_reuses_recent_values() {
        let cache = Arc::new(Mutex::new(None));
        let mut calls = 0;

        let first = with_cached_latest_openclaw_version(&cache, "macNative:default", 1_000, || {
            calls += 1;
            Some("2026.3.10".into())
        });
        let second =
            with_cached_latest_openclaw_version(&cache, "macNative:default", 30_000, || {
                calls += 1;
                Some("2026.3.11".into())
            });

        assert_eq!(first.as_deref(), Some("2026.3.10"));
        assert_eq!(second.as_deref(), Some("2026.3.10"));
        assert_eq!(calls, 1);
    }

    #[test]
    fn latest_version_cache_refreshes_after_ttl_expires() {
        let cache = Arc::new(Mutex::new(None));
        let mut calls = 0;

        let first = with_cached_latest_openclaw_version(&cache, "macNative:default", 1_000, || {
            calls += 1;
            Some("2026.3.10".into())
        });
        let second =
            with_cached_latest_openclaw_version(&cache, "macNative:default", 61_500, || {
                calls += 1;
                Some("2026.3.11".into())
            });

        assert_eq!(first.as_deref(), Some("2026.3.10"));
        assert_eq!(second.as_deref(), Some("2026.3.11"));
        assert_eq!(calls, 2);
    }

    #[test]
    fn cached_latest_version_still_marks_update_as_available() {
        assert!(update_available_from_versions(
            Some("OpenClaw 2026.3.8"),
            Some("2026.3.10"),
        ));
        assert!(!update_available_from_versions(
            Some("OpenClaw 2026.3.10"),
            Some("2026.3.10"),
        ));
        assert!(!update_available_from_versions(
            Some("OpenClaw 2026.3.10"),
            None
        ));
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
