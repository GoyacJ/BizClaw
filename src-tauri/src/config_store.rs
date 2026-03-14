use std::path::{Path, PathBuf};
use std::{fs, io::ErrorKind};

use anyhow::{anyhow, Result};

use crate::types::{PersistedSettings, UiPreferences};

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

#[derive(Debug, Clone)]
pub struct JsonUiPreferencesStore {
    path: PathBuf,
}

impl JsonUiPreferencesStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Option<UiPreferences>> {
        match fs::read_to_string(&self.path) {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, preferences: &UiPreferences) -> Result<()> {
        let content = serde_json::to_string_pretty(preferences)?;
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

    use super::{JsonSettingsStore, JsonUiPreferencesStore};
    use crate::types::{
        CompanyProfile, LocalePreference, PersistedSettings, TargetProfile, ThemePreference,
        UiPreferences, UserProfile,
    };

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

    #[test]
    fn saves_and_loads_ui_preferences_round_trip() {
        let temp = tempdir().unwrap();
        let store = JsonUiPreferencesStore::new(temp.path().join("ui-preferences.json"));
        let preferences = UiPreferences {
            theme: ThemePreference::System,
            locale: LocalePreference::EnUs,
        };

        store.save(&preferences).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded, Some(preferences));
    }

    #[test]
    fn loads_ui_preferences_from_frontend_locale_values() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("ui-preferences.json");
        std::fs::write(
            &path,
            r#"{
  "theme": "dark",
  "locale": "en-US"
}"#,
        )
        .unwrap();
        let store = JsonUiPreferencesStore::new(path);

        let loaded = store.load().unwrap();

        assert_eq!(
            loaded,
            Some(UiPreferences {
                theme: ThemePreference::Dark,
                locale: LocalePreference::EnUs,
            }),
        );
    }

    #[test]
    fn saves_ui_preferences_using_frontend_locale_values() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("ui-preferences.json");
        let store = JsonUiPreferencesStore::new(&path);

        store
            .save(&UiPreferences {
                theme: ThemePreference::Dark,
                locale: LocalePreference::EnUs,
            })
            .unwrap();

        let content = std::fs::read_to_string(path).unwrap();

        assert!(content.contains(r#""locale": "en-US""#));
    }

    #[test]
    fn loads_ui_preferences_with_system_theme() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("ui-preferences.json");
        std::fs::write(
            &path,
            r#"{
  "theme": "system",
  "locale": "zh-CN"
}"#,
        )
        .unwrap();
        let store = JsonUiPreferencesStore::new(path);

        let loaded = store.load().unwrap();

        assert_eq!(
            loaded,
            Some(UiPreferences {
                theme: ThemePreference::System,
                locale: LocalePreference::ZhCn,
            }),
        );
    }

    #[test]
    fn saves_ui_preferences_using_system_theme_value() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("ui-preferences.json");
        let store = JsonUiPreferencesStore::new(&path);

        store
            .save(&UiPreferences {
                theme: ThemePreference::System,
                locale: LocalePreference::EnUs,
            })
            .unwrap();

        let content = std::fs::read_to_string(path).unwrap();

        assert!(content.contains(r#""theme": "system""#));
    }

    #[test]
    fn loads_legacy_ui_preferences_locale_values() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("ui-preferences.json");
        std::fs::write(
            &path,
            r#"{
  "theme": "dark",
  "locale": "enUs"
}"#,
        )
        .unwrap();
        let store = JsonUiPreferencesStore::new(path);

        let loaded = store.load().unwrap();

        assert_eq!(
            loaded,
            Some(UiPreferences {
                theme: ThemePreference::Dark,
                locale: LocalePreference::EnUs,
            }),
        );
    }
}
