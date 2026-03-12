use anyhow::{anyhow, Result};

use crate::types::{CompanyProfile, PersistedSettings, UserProfile};

pub fn validate_company_profile(profile: &CompanyProfile) -> Result<()> {
    if profile.ssh_host.trim().is_empty() {
        return Err(anyhow!("SSH 主机地址不能为空"));
    }

    if profile.ssh_user.trim().is_empty() {
        return Err(anyhow!("SSH 用户名不能为空"));
    }

    if profile.remote_bind_host.trim().is_empty() {
        return Err(anyhow!("远端绑定地址不能为空"));
    }

    if profile.local_port == 0 || profile.remote_bind_port == 0 {
        return Err(anyhow!("端口必须在 1-65535 之间"));
    }

    Ok(())
}

pub fn validate_user_profile(profile: &UserProfile) -> Result<()> {
    if profile.display_name.trim().is_empty() {
        return Err(anyhow!("显示名称不能为空"));
    }

    Ok(())
}

pub fn saved_settings_are_complete(settings: &PersistedSettings) -> bool {
    validate_company_profile(&settings.company_profile).is_ok()
        && validate_user_profile(&settings.user_profile).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{saved_settings_are_complete, validate_company_profile, validate_user_profile};
    use crate::types::{CompanyProfile, PersistedSettings, UserProfile};

    #[test]
    fn accepts_valid_company_profile() {
        let profile = CompanyProfile {
            ssh_host: "gateway.example.com".into(),
            ssh_user: "bizclaw".into(),
            local_port: 32001,
            remote_bind_host: "localhost".into(),
            remote_bind_port: 32002,
        };

        assert!(validate_company_profile(&profile).is_ok());
    }

    #[test]
    fn rejects_empty_company_host() {
        let profile = CompanyProfile {
            ssh_host: String::new(),
            ssh_user: "bizclaw".into(),
            local_port: 32001,
            remote_bind_host: "localhost".into(),
            remote_bind_port: 32002,
        };

        let error = validate_company_profile(&profile).unwrap_err();
        assert!(error.to_string().contains("SSH"));
    }

    #[test]
    fn rejects_blank_display_name() {
        let profile = UserProfile {
            display_name: "   ".into(),
            auto_connect: true,
            run_in_background: true,
        };

        let error = validate_user_profile(&profile).unwrap_err();
        assert!(error.to_string().contains("显示名称"));
    }

    #[test]
    fn complete_saved_settings_are_usable() {
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
        };

        assert!(saved_settings_are_complete(&settings));
    }

    #[test]
    fn incomplete_saved_settings_are_not_usable() {
        let settings = PersistedSettings {
            company_profile: CompanyProfile {
                ssh_host: "gateway.example.com".into(),
                ssh_user: String::new(),
                local_port: 0,
                remote_bind_host: String::new(),
                remote_bind_port: 32002,
            },
            user_profile: UserProfile {
                display_name: " ".into(),
                auto_connect: true,
                run_in_background: true,
            },
        };

        assert!(!saved_settings_are_complete(&settings));
    }
}
