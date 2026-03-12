use crate::types::{CompanyProfile, UserProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

pub fn build_ssh_command(
    profile: &CompanyProfile,
    password_auth: Option<(&str, &str)>,
) -> CommandSpec {
    let mut args = vec!["-N".into()];
    let mut envs = Vec::new();

    if let Some((askpass_program, password)) = password_auth {
        args.extend([
            "-o".into(),
            "ExitOnForwardFailure=yes".into(),
            "-o".into(),
            "PreferredAuthentications=password,keyboard-interactive".into(),
            "-o".into(),
            "PubkeyAuthentication=no".into(),
            "-o".into(),
            "NumberOfPasswordPrompts=1".into(),
        ]);
        envs.extend([
            ("SSH_ASKPASS".into(), askpass_program.into()),
            ("SSH_ASKPASS_REQUIRE".into(), "force".into()),
            ("DISPLAY".into(), "bizclaw:0".into()),
            ("BIZCLAW_SSH_ASKPASS".into(), "1".into()),
            ("BIZCLAW_SSH_PASSWORD".into(), password.into()),
        ]);
    } else {
        args.extend([
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ExitOnForwardFailure=yes".into(),
        ]);
    }

    args.extend([
        "-L".into(),
        format!(
            "{}:{}:{}",
            profile.local_port, profile.remote_bind_host, profile.remote_bind_port
        ),
        format!("{}@{}", profile.ssh_user, profile.ssh_host),
    ]);

    CommandSpec {
        program: "ssh".into(),
        args,
        envs,
    }
}

pub fn build_openclaw_command(
    _profile: &CompanyProfile,
    _user_profile: &UserProfile,
    _token: &str,
) -> CommandSpec {
    CommandSpec {
        program: "openclaw".into(),
        args: vec![
            "node".into(),
            "run".into(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            _profile.local_port.to_string(),
            "--display-name".into(),
            _user_profile.display_name.clone(),
        ],
        envs: vec![("OPENCLAW_GATEWAY_TOKEN".into(), _token.into())],
    }
}

#[cfg(test)]
mod tests {
    use super::{build_openclaw_command, build_ssh_command};
    use crate::types::{CompanyProfile, UserProfile};

    fn sample_company_profile() -> CompanyProfile {
        CompanyProfile {
            ssh_host: "gateway.example.com".into(),
            ssh_user: "bizclaw".into(),
            local_port: 32001,
            remote_bind_host: "localhost".into(),
            remote_bind_port: 32002,
        }
    }

    #[test]
    fn ssh_command_matches_company_profile_values() {
        let profile = sample_company_profile();

        let command = build_ssh_command(&profile, None);

        assert_eq!(command.program, "ssh");
        assert_eq!(
            command.args,
            vec![
                "-N",
                "-o",
                "BatchMode=yes",
                "-o",
                "ExitOnForwardFailure=yes",
                "-L",
                "32001:localhost:32002",
                "bizclaw@gateway.example.com",
            ]
        );
        assert!(command.envs.is_empty());
    }

    #[test]
    fn ssh_command_can_switch_to_password_auth_with_askpass() {
        let profile = sample_company_profile();

        let command = build_ssh_command(
            &profile,
            Some(("/tmp/bizclaw-askpass", "ssh-password")),
        );

        assert_eq!(command.program, "ssh");
        assert_eq!(
            command.args,
            vec![
                "-N",
                "-o",
                "ExitOnForwardFailure=yes",
                "-o",
                "PreferredAuthentications=password,keyboard-interactive",
                "-o",
                "PubkeyAuthentication=no",
                "-o",
                "NumberOfPasswordPrompts=1",
                "-L",
                "32001:localhost:32002",
                "bizclaw@gateway.example.com",
            ]
        );
        assert_eq!(
            command.envs,
            vec![
                ("SSH_ASKPASS".into(), "/tmp/bizclaw-askpass".into()),
                ("SSH_ASKPASS_REQUIRE".into(), "force".into()),
                ("DISPLAY".into(), "bizclaw:0".into()),
                ("BIZCLAW_SSH_ASKPASS".into(), "1".into()),
                ("BIZCLAW_SSH_PASSWORD".into(), "ssh-password".into()),
            ]
        );
    }

    #[test]
    fn openclaw_command_uses_local_tunnel_and_display_name() {
        let company = sample_company_profile();
        let user = UserProfile {
            display_name: "BizClaw Mac".into(),
            auto_connect: true,
            run_in_background: true,
        };

        let command = build_openclaw_command(&company, &user, "gateway-token");

        assert_eq!(command.program, "openclaw");
        assert_eq!(
            command.args,
            vec![
                "node",
                "run",
                "--host",
                "127.0.0.1",
                "--port",
                "32001",
                "--display-name",
                "BizClaw Mac",
            ]
        );
        assert_eq!(
            command.envs,
            vec![("OPENCLAW_GATEWAY_TOKEN".into(), "gateway-token".into())]
        );
    }
}
