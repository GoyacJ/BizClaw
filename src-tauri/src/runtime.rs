use crate::types::{CompanyProfile, UserProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

pub fn build_ssh_command(_profile: &CompanyProfile) -> CommandSpec {
    CommandSpec {
        program: "ssh".into(),
        args: vec![
            "-N".into(),
            "-L".into(),
            format!(
                "{}:{}:{}",
                _profile.local_port, _profile.remote_bind_host, _profile.remote_bind_port
            ),
            format!("{}@{}", _profile.ssh_user, _profile.ssh_host),
        ],
        envs: Vec::new(),
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

        let command = build_ssh_command(&profile);

        assert_eq!(command.program, "ssh");
        assert_eq!(
            command.args,
            vec!["-N", "-L", "32001:localhost:32002", "bizclaw@gateway.example.com",]
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
