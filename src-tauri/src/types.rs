use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSettings {
    pub company_profile: CompanyProfile,
    pub user_profile: UserProfile,
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
    pub last_error: Option<String>,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            phase: RuntimePhase::Checking,
            ssh_connected: false,
            node_connected: false,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSnapshot {
    pub os: String,
    pub ssh_installed: bool,
    pub openclaw_installed: bool,
    pub npm_installed: bool,
    pub pnpm_installed: bool,
    pub has_saved_profile: bool,
    pub has_saved_token: bool,
    pub saved_settings: Option<PersistedSettings>,
    pub runtime_status: RuntimeStatus,
    pub install_recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub prefer_official: bool,
    pub allow_elevation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub strategy: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub needs_elevation: bool,
    pub manual_url: String,
    pub follow_up: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub source: String,
    pub level: String,
    pub message: String,
    pub timestamp_ms: u64,
}
