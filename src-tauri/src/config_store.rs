use std::path::{Path, PathBuf};
use std::{fs, io::ErrorKind};

use anyhow::{anyhow, Result};

use crate::types::PersistedSettings;

#[derive(Debug, Clone)]
pub struct JsonSettingsStore {
    path: PathBuf,
}

impl JsonSettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Option<PersistedSettings>> {
        match fs::read_to_string(&self.path) {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, settings: &PersistedSettings) -> Result<()> {
        let content = serde_json::to_string_pretty(settings)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        } else {
            return Err(anyhow!("配置文件路径缺少父目录"));
        }

        fs::write(&self.path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::JsonSettingsStore;
    use crate::types::{CompanyProfile, PersistedSettings, TargetProfile, UserProfile};

    #[test]
    fn saves_and_loads_settings_round_trip() {
        let temp = tempdir().unwrap();
        let store = JsonSettingsStore::new(temp.path().join("settings.json"));
        let settings = PersistedSettings {
            company_profile: CompanyProfile {
                ssh_host: "gateway.example.com".into(),
                ssh_user: "bizclaw".into(),
                local_port: 32001,
                remote_bind_host: "localhost".into(),
                remote_bind_port: 32002,
            },
            user_profile: UserProfile {
                display_name: "BizClaw Mac".into(),
                auto_connect: true,
                run_in_background: true,
            },
            target_profile: TargetProfile::default(),
        };

        store.save(&settings).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded, Some(settings));
    }

    #[test]
    fn missing_settings_file_returns_none() {
        let temp = tempdir().unwrap();
        let store = JsonSettingsStore::new(temp.path().join("missing.json"));

        assert_eq!(store.load().unwrap(), None);
    }

    #[test]
    fn saved_settings_file_never_contains_gateway_token() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("settings.json");
        let store = JsonSettingsStore::new(&path);
        let settings = PersistedSettings {
            company_profile: CompanyProfile {
                ssh_host: "gateway.example.com".into(),
                ssh_user: "bizclaw".into(),
                local_port: 32001,
                remote_bind_host: "localhost".into(),
                remote_bind_port: 32002,
            },
            user_profile: UserProfile {
                display_name: "BizClaw Mac".into(),
                auto_connect: false,
                run_in_background: true,
            },
            target_profile: TargetProfile::default(),
        };

        store.save(&settings).unwrap();
        let content = std::fs::read_to_string(path).unwrap();

        assert!(!content.contains("OPENCLAW_GATEWAY_TOKEN"));
        assert!(!content.contains("gateway-token"));
    }
}
