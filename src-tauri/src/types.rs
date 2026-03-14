use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeTarget {
    MacNative,
    WindowsWsl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetProfile {
    #[serde(default = "default_wsl_distro")]
    pub wsl_distro: String,
}

impl Default for TargetProfile {
    fn default() -> Self {
        Self {
            wsl_distro: default_wsl_distro(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompanyProfile {
    pub ssh_host: String,
    pub ssh_user: String,
    pub local_port: u16,
    pub remote_bind_host: String,
    pub remote_bind_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub display_name: String,
    pub auto_connect: bool,
    pub run_in_background: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    Light,
    Dark,
    System,
}

impl Default for ThemePreference {
    fn default() -> Self {
        Self::Light
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocalePreference {
    #[serde(rename = "zh-CN", alias = "zhCn")]
    ZhCn,
    #[serde(rename = "en-US", alias = "enUs")]
    EnUs,
}

impl Default for LocalePreference {
    fn default() -> Self {
        Self::ZhCn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiPreferences {
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub locale: LocalePreference,
    #[serde(default)]
    pub sidebar_collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSettings {
    pub company_profile: CompanyProfile,
    pub user_profile: UserProfile,
    #[serde(default)]
    pub target_profile: TargetProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimePhase {
    Checking,
    InstallNeeded,
    Installing,
    ManualWait,
    Configured,
    Connecting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub phase: RuntimePhase,
    pub ssh_connected: bool,
    pub node_connected: bool,
    pub gateway_connected: bool,
    pub last_error: Option<String>,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            phase: RuntimePhase::Checking,
            ssh_connected: false,
            node_connected: false,
            gateway_connected: false,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSnapshot {
    pub os: String,
    pub runtime_target: RuntimeTarget,
    pub host_ssh_installed: bool,
    pub target_ssh_installed: bool,
    pub openclaw_installed: bool,
    pub openclaw_version: Option<String>,
    pub latest_openclaw_version: Option<String>,
    pub update_available: bool,
    pub has_saved_profile: bool,
    pub token_status: TokenStatus,
    pub token_status_message: Option<String>,
    #[serde(default)]
    pub ui_preferences: UiPreferences,
    pub saved_settings: Option<PersistedSettings>,
    pub runtime_status: RuntimeStatus,
    pub install_recommendation: String,
    pub wsl_status: Option<WslStatus>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TokenStatus {
    Saved,
    Missing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub prefer_official: bool,
    pub allow_elevation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub kind: OperationKind,
    pub strategy: String,
    pub success: bool,
    pub step: OperationStep,
    pub stdout: String,
    pub stderr: String,
    pub needs_elevation: bool,
    pub manual_url: String,
    pub follow_up: String,
    #[serde(default)]
    pub remediation: Option<OperationRemediation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationRemediation {
    pub kind: OperationRemediationKind,
    pub url_target: Option<SupportUrlTarget>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationRemediationKind {
    RequestElevation,
    InstallHomebrew,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SupportUrlTarget {
    #[serde(rename = "openclawManual")]
    OpenClawManual,
    #[serde(rename = "homebrewInstall")]
    HomebrewInstall,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationTaskPhase {
    Idle,
    Running,
    Cancelling,
    Success,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationTaskSnapshot {
    pub phase: OperationTaskPhase,
    pub kind: Option<OperationKind>,
    pub step: Option<OperationStep>,
    pub can_stop: bool,
    pub last_result: Option<OperationResult>,
    pub started_at: Option<u64>,
    pub ended_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub source: String,
    pub level: String,
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WslStatus {
    pub available: bool,
    pub distro_name: String,
    pub distro_installed: bool,
    pub ready: bool,
    pub needs_reboot: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    Install,
    CheckUpdate,
    Update,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationStep {
    Detect,
    BootstrapWsl,
    EnsureSsh,
    InstallOpenClaw,
    CheckUpdate,
    UpdateOpenClaw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationEventStatus {
    Running,
    Success,
    Error,
    Log,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationEventSource {
    System,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub kind: OperationKind,
    pub step: OperationStep,
    pub status: OperationEventStatus,
    pub source: OperationEventSource,
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionTestStep {
    Save,
    SshTunnel,
    GatewayProbe,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionTestEventStatus {
    Running,
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestEvent {
    pub step: ConnectionTestStep,
    pub status: ConnectionTestEventStatus,
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub success: bool,
    pub step: ConnectionTestStep,
    pub summary: String,
    pub stdout: String,
    pub stderr: String,
}

fn default_wsl_distro() -> String {
    "Ubuntu".into()
}
