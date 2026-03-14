use anyhow::{anyhow, Result};

use crate::types::{
    CompanyProfile, LocalePreference, PersistedSettings, TargetProfile, UserProfile,
};

pub fn validate_company_profile(profile: &CompanyProfile) -> Result<()> {
    validate_company_profile_with_locale(profile, LocalePreference::ZhCn)
}

pub fn validate_company_profile_with_locale(
    profile: &CompanyProfile,
    locale: LocalePreference,
) -> Result<()> {
    if profile.ssh_host.trim().is_empty() {
        return Err(anyhow!(locale_text(
            locale,
            "SSH 主机地址不能为空",
            "SSH host cannot be empty"
        )));
    }

    if profile.ssh_user.trim().is_empty() {
        return Err(anyhow!(locale_text(
            locale,
            "SSH 用户名不能为空",
            "SSH username cannot be empty"
        )));
    }

    if profile.remote_bind_host.trim().is_empty() {
        return Err(anyhow!(locale_text(
            locale,
            "远端绑定地址不能为空",
            "Remote bind host cannot be empty"
        )));
    }

    if profile.local_port == 0 || profile.remote_bind_port == 0 {
        return Err(anyhow!(locale_text(
            locale,
            "端口必须在 1-65535 之间",
            "Ports must be between 1 and 65535"
        )));
    }

    Ok(())
}

pub fn validate_user_profile(profile: &UserProfile) -> Result<()> {
    validate_user_profile_with_locale(profile, LocalePreference::ZhCn)
}

pub fn validate_user_profile_with_locale(
    profile: &UserProfile,
    locale: LocalePreference,
) -> Result<()> {
    if profile.display_name.trim().is_empty() {
        return Err(anyhow!(locale_text(
            locale,
            "显示名称不能为空",
            "Display name cannot be empty"
        )));
    }

    Ok(())
}

pub fn validate_target_profile(profile: &TargetProfile) -> Result<()> {
    validate_target_profile_with_locale(profile, LocalePreference::ZhCn)
}

pub fn validate_target_profile_with_locale(
    profile: &TargetProfile,
    locale: LocalePreference,
) -> Result<()> {
    if profile.wsl_distro.trim().is_empty() {
        return Err(anyhow!(locale_text(
            locale,
            "WSL 发行版不能为空",
            "WSL distro cannot be empty"
        )));
    }

    Ok(())
}

pub fn saved_settings_are_complete(settings: &PersistedSettings) -> bool {
    validate_company_profile(&settings.company_profile).is_ok()
        && validate_user_profile(&settings.user_profile).is_ok()
        && validate_target_profile(&settings.target_profile).is_ok()
}

fn locale_text(locale: LocalePreference, zh: &'static str, en: &'static str) -> &'static str {
    if matches!(locale, LocalePreference::EnUs) {
        en
    } else {
        zh
    }
}

#[cfg(test)]
mod tests {
    use super::{
        saved_settings_are_complete, validate_company_profile,
        validate_company_profile_with_locale, validate_target_profile, validate_user_profile,
        validate_user_profile_with_locale,
    };
    use crate::types::{
        CompanyProfile, LocalePreference, PersistedSettings, TargetProfile, UserProfile,
    };

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
    fn emits_english_validation_errors_when_requested() {
        let company_error = validate_company_profile_with_locale(
            &CompanyProfile {
                ssh_host: String::new(),
                ssh_user: "bizclaw".into(),
                local_port: 32001,
                remote_bind_host: "localhost".into(),
                remote_bind_port: 32002,
            },
            LocalePreference::EnUs,
        )
        .unwrap_err();
        let user_error = validate_user_profile_with_locale(
            &UserProfile {
                display_name: " ".into(),
                auto_connect: true,
                run_in_background: true,
            },
            LocalePreference::EnUs,
        )
        .unwrap_err();

        assert_eq!(company_error.to_string(), "SSH host cannot be empty");
        assert_eq!(user_error.to_string(), "Display name cannot be empty");
    }

    #[test]
    fn rejects_blank_wsl_distro() {
        let error = validate_target_profile(&TargetProfile {
            wsl_distro: " ".into(),
        })
        .unwrap_err();

        assert!(error.to_string().contains("WSL"));
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
            target_profile: TargetProfile::default(),
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
            target_profile: TargetProfile::default(),
        };

        assert!(!saved_settings_are_complete(&settings));
    }
}
