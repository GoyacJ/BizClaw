#[cfg(target_os = "windows")]
use crate::install::windows_ssh_executable_path;
use crate::types::{CompanyProfile, TargetProfile, UserProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

pub fn build_native_ssh_command(
    profile: &CompanyProfile,
    password_auth: Option<(&str, &str)>,
) -> CommandSpec {
    let mut args = vec!["-N".into()];
    let mut envs = Vec::new();

    if let Some((askpass_program, password)) = password_auth {
        args.extend([
            "-o".into(),
            "StrictHostKeyChecking=accept-new".into(),
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
            "StrictHostKeyChecking=accept-new".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ExitOnForwardFailure=yes".into(),
        ]);
    }

    args.extend(ssh_forward_args(profile));

    CommandSpec {
        program: native_ssh_program(),
        args,
        envs,
    }
}

fn native_ssh_program() -> String {
    #[cfg(target_os = "windows")]
    {
        return windows_ssh_executable_path()
            .unwrap_or_else(|| "ssh".into())
            .to_string_lossy()
            .into_owned();
    }

    #[cfg(not(target_os = "windows"))]
    {
        "ssh".into()
    }
}

pub fn build_wsl_ssh_command(
    target_profile: &TargetProfile,
    profile: &CompanyProfile,
    password: Option<&str>,
) -> CommandSpec {
    let shell_payload = if password.is_some() {
        format!(
            "ASKPASS_SCRIPT=$(mktemp)\ncat <<'EOF' > \"$ASKPASS_SCRIPT\"\n#!/bin/sh\nprintf '%s' \"$BIZCLAW_SSH_PASSWORD\"\nEOF\nchmod 700 \"$ASKPASS_SCRIPT\"\ntrap 'rm -f \"$ASKPASS_SCRIPT\"' EXIT\nexport SSH_ASKPASS=\"$ASKPASS_SCRIPT\"\nexport SSH_ASKPASS_REQUIRE=force\nexport DISPLAY=bizclaw:0\nexec ssh -N -o ExitOnForwardFailure=yes -o PreferredAuthentications=password,keyboard-interactive -o PubkeyAuthentication=no -o NumberOfPasswordPrompts=1 -L {} {}",
            sh_quote(&ssh_forward_target(profile)),
            sh_quote(&ssh_remote_target(profile))
        )
    } else {
        format!(
            "exec ssh -N -o BatchMode=yes -o ExitOnForwardFailure=yes -L {} {}",
            sh_quote(&ssh_forward_target(profile)),
            sh_quote(&ssh_remote_target(profile))
        )
    };

    let mut envs = Vec::new();
    if let Some(password) = password {
        envs.push(("BIZCLAW_SSH_PASSWORD".into(), password.into()));
        append_wslenv(&mut envs, &["BIZCLAW_SSH_PASSWORD"]);
    }

    build_wsl_shell_command(target_profile, shell_payload, envs)
}

pub fn build_native_openclaw_command(
    profile: &CompanyProfile,
    user_profile: &UserProfile,
    token: &str,
) -> CommandSpec {
    CommandSpec {
        program: "openclaw".into(),
        args: openclaw_args(profile, user_profile),
        envs: vec![("OPENCLAW_GATEWAY_TOKEN".into(), token.into())],
    }
}

pub fn build_wsl_openclaw_command(
    target_profile: &TargetProfile,
    profile: &CompanyProfile,
    user_profile: &UserProfile,
    token: &str,
) -> CommandSpec {
    let shell_payload = format!(
        "exec openclaw node run --host 127.0.0.1 --port {} --display-name {}",
        profile.local_port,
        sh_quote(&user_profile.display_name)
    );
    let mut envs = vec![("OPENCLAW_GATEWAY_TOKEN".into(), token.into())];
    append_wslenv(&mut envs, &["OPENCLAW_GATEWAY_TOKEN"]);

    build_wsl_shell_command(target_profile, shell_payload, envs)
}

pub fn build_native_gateway_status_command(
    profile: &CompanyProfile,
    token: &str,
    timeout_ms: u64,
) -> CommandSpec {
    CommandSpec {
        program: "openclaw".into(),
        args: vec![
            "gateway".into(),
            "status".into(),
            "--json".into(),
            "--url".into(),
            gateway_status_url(profile),
            "--token".into(),
            token.into(),
            "--timeout".into(),
            timeout_ms.to_string(),
        ],
        envs: Vec::new(),
    }
}

pub fn build_wsl_gateway_status_command(
    target_profile: &TargetProfile,
    profile: &CompanyProfile,
    token: &str,
    timeout_ms: u64,
) -> CommandSpec {
    let shell_payload = format!(
        "exec openclaw gateway status --json --url {} --token {} --timeout {}",
        sh_quote(&gateway_status_url(profile)),
        sh_quote(token),
        timeout_ms
    );

    build_wsl_shell_command(target_profile, shell_payload, Vec::new())
}

fn build_wsl_shell_command(
    target_profile: &TargetProfile,
    shell_payload: String,
    envs: Vec<(String, String)>,
) -> CommandSpec {
    CommandSpec {
        program: "wsl.exe".into(),
        args: vec![
            "-d".into(),
            target_profile.wsl_distro.clone(),
            "--".into(),
            "bash".into(),
            "-lc".into(),
            shell_payload,
        ],
        envs,
    }
}

fn openclaw_args(profile: &CompanyProfile, user_profile: &UserProfile) -> Vec<String> {
    vec![
        "node".into(),
        "run".into(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        profile.local_port.to_string(),
        "--display-name".into(),
        user_profile.display_name.clone(),
    ]
}

fn ssh_forward_args(profile: &CompanyProfile) -> Vec<String> {
    vec![
        "-L".into(),
        ssh_forward_target(profile),
        ssh_remote_target(profile),
    ]
}

fn ssh_forward_target(profile: &CompanyProfile) -> String {
    format!(
        "{}:{}:{}",
        profile.local_port, profile.remote_bind_host, profile.remote_bind_port
    )
}

fn ssh_remote_target(profile: &CompanyProfile) -> String {
    format!("{}@{}", profile.ssh_user, profile.ssh_host)
}

fn gateway_status_url(profile: &CompanyProfile) -> String {
    format!("ws://127.0.0.1:{}", profile.local_port)
}

fn append_wslenv(envs: &mut Vec<(String, String)>, keys: &[&str]) {
    envs.push(("WSLENV".into(), keys.join(":")));
}

fn sh_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::{
        build_native_gateway_status_command, build_native_openclaw_command,
        build_native_ssh_command, build_wsl_gateway_status_command, build_wsl_openclaw_command,
        build_wsl_ssh_command,
    };
    use crate::types::{CompanyProfile, TargetProfile, UserProfile};

    fn sample_company_profile() -> CompanyProfile {
        CompanyProfile {
            ssh_host: "gateway.example.com".into(),
            ssh_user: "bizclaw".into(),
            local_port: 32001,
            remote_bind_host: "127.0.0.1".into(),
            remote_bind_port: 32002,
        }
    }

    #[test]
    fn native_ssh_command_matches_company_profile_values() {
        let profile = sample_company_profile();

        let command = build_native_ssh_command(&profile, None);

        #[cfg(target_os = "windows")]
        assert!(command.program.to_ascii_lowercase().ends_with("ssh.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(command.program, "ssh");
        assert_eq!(
            command.args,
            vec![
                "-N",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "BatchMode=yes",
                "-o",
                "ExitOnForwardFailure=yes",
                "-L",
                "32001:127.0.0.1:32002",
                "bizclaw@gateway.example.com",
            ]
        );
        assert!(command.envs.is_empty());
    }

    #[test]
    fn native_ssh_command_can_switch_to_password_auth_with_askpass() {
        let profile = sample_company_profile();

        let command =
            build_native_ssh_command(&profile, Some(("/tmp/bizclaw-askpass", "ssh-password")));

        #[cfg(target_os = "windows")]
        assert!(command.program.to_ascii_lowercase().ends_with("ssh.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(command.program, "ssh");
        assert_eq!(
            command.args,
            vec![
                "-N",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "ExitOnForwardFailure=yes",
                "-o",
                "PreferredAuthentications=password,keyboard-interactive",
                "-o",
                "PubkeyAuthentication=no",
                "-o",
                "NumberOfPasswordPrompts=1",
                "-L",
                "32001:127.0.0.1:32002",
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
    fn native_openclaw_command_uses_local_tunnel_and_display_name() {
        let company = sample_company_profile();
        let user = UserProfile {
            display_name: "BizClaw Mac".into(),
            auto_connect: true,
            run_in_background: true,
        };

        let command = build_native_openclaw_command(&company, &user, "gateway-token");

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

    #[test]
    fn wsl_openclaw_command_wraps_node_run_in_wsl() {
        let company = sample_company_profile();
        let user = UserProfile {
            display_name: "BizClaw Windows".into(),
            auto_connect: true,
            run_in_background: true,
        };
        let target = TargetProfile {
            wsl_distro: "Ubuntu".into(),
        };

        let command = build_wsl_openclaw_command(&target, &company, &user, "gateway-token");

        assert_eq!(command.program, "wsl.exe");
        assert_eq!(command.args[..5], ["-d", "Ubuntu", "--", "bash", "-lc"]);
        assert!(command
            .args
            .last()
            .expect("wsl shell payload")
            .contains("openclaw node run --host 127.0.0.1 --port 32001"));
        assert_eq!(
            command.envs,
            vec![
                ("OPENCLAW_GATEWAY_TOKEN".into(), "gateway-token".into()),
                ("WSLENV".into(), "OPENCLAW_GATEWAY_TOKEN".into()),
            ]
        );
    }

    #[test]
    fn wsl_ssh_command_builds_ephemeral_askpass_script() {
        let profile = sample_company_profile();
        let target = TargetProfile::default();

        let command = build_wsl_ssh_command(&target, &profile, Some("ssh-password"));

        assert_eq!(command.program, "wsl.exe");
        assert_eq!(command.args[..5], ["-d", "Ubuntu", "--", "bash", "-lc"]);
        let shell_payload = command.args.last().expect("wsl shell payload");
        assert!(shell_payload.contains("mktemp"));
        assert!(shell_payload.contains("SSH_ASKPASS"));
        assert!(shell_payload.contains("ssh -N"));
        assert!(shell_payload.contains("32001:127.0.0.1:32002"));
        assert_eq!(
            command.envs,
            vec![
                ("BIZCLAW_SSH_PASSWORD".into(), "ssh-password".into()),
                ("WSLENV".into(), "BIZCLAW_SSH_PASSWORD".into()),
            ]
        );
    }

    #[test]
    fn native_gateway_status_command_targets_loopback_and_token() {
        let profile = sample_company_profile();

        let command = build_native_gateway_status_command(&profile, "gateway-token", 8_000);

        assert_eq!(command.program, "openclaw");
        assert_eq!(
            command.args,
            vec![
                "gateway",
                "status",
                "--json",
                "--url",
                "ws://127.0.0.1:32001",
                "--token",
                "gateway-token",
                "--timeout",
                "8000",
            ]
        );
        assert!(command.envs.is_empty());
    }

    #[test]
    fn wsl_gateway_status_command_wraps_probe_in_wsl_shell() {
        let profile = sample_company_profile();
        let target = TargetProfile::default();

        let command = build_wsl_gateway_status_command(&target, &profile, "gateway-token", 8_000);

        assert_eq!(command.program, "wsl.exe");
        assert_eq!(command.args[..5], ["-d", "Ubuntu", "--", "bash", "-lc"]);
        let shell_payload = command.args.last().expect("wsl shell payload");
        assert!(shell_payload.contains("openclaw gateway status --json"));
        assert!(shell_payload.contains("ws://127.0.0.1:32001"));
        assert!(shell_payload.contains("gateway-token"));
        assert!(shell_payload.contains("--timeout 8000"));
    }
}
