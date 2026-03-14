use std::{env, process::Command};

use anyhow::{anyhow, Result};

use crate::types::{RuntimeTarget, TargetProfile, WslStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub strategy: &'static str,
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

pub const MANUAL_INSTALL_URL: &str = "https://docs.openclaw.ai/install";

pub fn current_platform() -> Result<Platform> {
    match env::consts::OS {
        "macos" => Ok(Platform::MacOs),
        "windows" => Ok(Platform::Windows),
        other => Err(anyhow!("暂不支持的系统: {other}")),
    }
}

pub fn default_runtime_target(platform: Platform) -> RuntimeTarget {
    match platform {
        Platform::MacOs => RuntimeTarget::MacNative,
        Platform::Windows => RuntimeTarget::WindowsWsl,
    }
}

pub fn command_available(name: &str) -> bool {
    which::which(name).is_ok()
}

pub fn target_command_available(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
    name: &str,
) -> bool {
    match target {
        RuntimeTarget::MacNative => command_available(name),
        RuntimeTarget::WindowsWsl => {
            let status = detect_wsl_status(target_profile);
            status.ready
                && run_wsl_command(
                    target_profile,
                    &format!("command -v {} >/dev/null 2>&1", sh_quote(name)),
                )
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
    }
}

pub fn official_install_plan(platform: Platform) -> InstallPlan {
    match platform {
        Platform::MacOs => InstallPlan {
            strategy: "official",
            program: "bash".into(),
            args: vec![
                "-lc".into(),
                "curl -fsSL --proto '=https' --tlsv1.2 https://openclaw.ai/install.sh | bash -s -- --no-prompt --no-onboard".into(),
            ],
            envs: Vec::new(),
        },
        Platform::Windows => wsl_bootstrap_plan(&TargetProfile::default()),
    }
}

pub fn official_install_plan_for_target(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> InstallPlan {
    match target {
        RuntimeTarget::MacNative => official_install_plan(Platform::MacOs),
        RuntimeTarget::WindowsWsl => wsl_openclaw_install_plan(target_profile),
    }
}

pub fn install_plans_for_target(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
    prefer_official: bool,
    has_npm: bool,
    has_pnpm: bool,
) -> Vec<InstallPlan> {
    let mut plans = Vec::new();
    if prefer_official {
        plans.push(official_install_plan_for_target(
            target.clone(),
            target_profile,
        ));
    }
    plans.extend(fallback_install_plans_for_target(
        target,
        target_profile,
        has_npm,
        has_pnpm,
    ));
    plans
}

pub fn update_plans_for_target(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
    prefer_official: bool,
    has_npm: bool,
    has_pnpm: bool,
) -> Vec<InstallPlan> {
    install_plans_for_target(target, target_profile, prefer_official, has_npm, has_pnpm)
}

pub fn fallback_install_plans(has_npm: bool, has_pnpm: bool) -> Vec<InstallPlan> {
    let mut plans = Vec::new();

    if has_npm {
        plans.push(InstallPlan {
            strategy: "npm",
            program: "npm".into(),
            args: vec!["install".into(), "-g".into(), "openclaw@latest".into()],
            envs: Vec::new(),
        });
    }

    if has_pnpm {
        plans.push(InstallPlan {
            strategy: "pnpm",
            program: "pnpm".into(),
            args: vec!["add".into(), "-g".into(), "openclaw@latest".into()],
            envs: Vec::new(),
        });
    }

    plans
}

pub fn fallback_install_plans_for_target(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
    has_npm: bool,
    has_pnpm: bool,
) -> Vec<InstallPlan> {
    match target {
        RuntimeTarget::MacNative => fallback_install_plans(has_npm, has_pnpm),
        RuntimeTarget::WindowsWsl => {
            let mut plans = Vec::new();
            if has_npm {
                plans.push(wsl_shell_plan(
                    "npm",
                    target_profile,
                    "npm install -g openclaw@latest",
                ));
            }
            if has_pnpm {
                plans.push(wsl_shell_plan(
                    "pnpm",
                    target_profile,
                    "pnpm add -g openclaw@latest",
                ));
            }
            plans
        }
    }
}

pub fn elevated_install_plan(plan: &InstallPlan, platform: Platform) -> Option<InstallPlan> {
    match platform {
        Platform::MacOs => Some(InstallPlan {
            strategy: "official-elevated",
            program: "osascript".into(),
            args: vec![
                "-e".into(),
                format!(
                    "do shell script {} with administrator privileges",
                    apple_quote(&format!("{} {}", plan.program, plan.args.join(" ")))
                ),
            ],
            envs: plan.envs.clone(),
        }),
        Platform::Windows => Some(InstallPlan {
            strategy: "official-elevated",
            program: "powershell".into(),
            args: vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-Command".into(),
                format!(
                    "$proc = Start-Process powershell -Verb RunAs -PassThru -Wait -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-Command', {}); exit $proc.ExitCode",
                    ps_quote(&format!("{} {}", plan.program, plan.args.join(" ")))
                ),
            ],
            envs: plan.envs.clone(),
        }),
    }
}

pub fn detect_wsl_status(target_profile: &TargetProfile) -> WslStatus {
    if !command_available("wsl") && !command_available("wsl.exe") {
        return WslStatus {
            available: false,
            distro_name: target_profile.wsl_distro.clone(),
            distro_installed: false,
            ready: false,
            needs_reboot: false,
            message: Some("未检测到 WSL，可通过自动安装继续。".into()),
        };
    }

    let listed = listed_wsl_distros().unwrap_or_default();
    let distro_installed = listed
        .iter()
        .any(|entry| entry.eq_ignore_ascii_case(&target_profile.wsl_distro));

    WslStatus {
        available: true,
        distro_name: target_profile.wsl_distro.clone(),
        distro_installed,
        ready: distro_installed,
        needs_reboot: false,
        message: if distro_installed {
            None
        } else {
            Some(format!(
                "尚未检测到 {}，可由 BizClaw 自动引导安装。",
                target_profile.wsl_distro
            ))
        },
    }
}

pub fn read_openclaw_version(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> Option<String> {
    read_first_non_empty_line(match target {
        RuntimeTarget::MacNative => Command::new("openclaw").arg("--version").output().ok()?,
        RuntimeTarget::WindowsWsl => run_wsl_command(target_profile, "openclaw --version").ok()?,
    })
}

pub fn read_latest_openclaw_version(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> Option<String> {
    match target {
        RuntimeTarget::MacNative => {
            if command_available("npm") {
                return read_first_non_empty_line(
                    Command::new("npm")
                        .args(["view", "openclaw", "version"])
                        .output()
                        .ok()?,
                );
            }

            if command_available("pnpm") {
                return read_first_non_empty_line(
                    Command::new("pnpm")
                        .args(["view", "openclaw", "version"])
                        .output()
                        .ok()?,
                );
            }

            None
        }
        RuntimeTarget::WindowsWsl => {
            if !detect_wsl_status(target_profile).ready {
                return None;
            }
            read_first_non_empty_line(
                run_wsl_command(
                    target_profile,
                    "if command -v npm >/dev/null 2>&1; then npm view openclaw version; elif command -v pnpm >/dev/null 2>&1; then pnpm view openclaw version; fi",
                )
                .ok()?,
            )
        }
    }
}

pub fn compare_versions(installed: &str, latest: &str) -> bool {
    let installed_segments = version_segments(installed);
    let latest_segments = version_segments(latest);
    if installed_segments.is_empty() || latest_segments.is_empty() {
        return installed.trim() != latest.trim();
    }

    latest_segments > installed_segments
}

pub fn looks_like_permission_error(output: &str) -> bool {
    let normalized = output.to_lowercase();
    normalized.contains("permission denied")
        || normalized.contains("requires elevation")
        || normalized.contains("administrator privileges")
        || normalized.contains("access is denied")
}

pub fn wsl_bootstrap_plan(target_profile: &TargetProfile) -> InstallPlan {
    InstallPlan {
        strategy: "bootstrap-wsl",
        program: "wsl.exe".into(),
        args: vec![
            "--install".into(),
            "-d".into(),
            target_profile.wsl_distro.clone(),
        ],
        envs: Vec::new(),
    }
}

pub fn wsl_ensure_ssh_plan(target_profile: &TargetProfile) -> InstallPlan {
    wsl_shell_plan(
        "ensure-ssh",
        target_profile,
        "if command -v ssh >/dev/null 2>&1; then echo 'OpenSSH already available'; else sudo apt-get update && sudo apt-get install -y openssh-client; fi",
    )
}

pub fn macos_ensure_ssh_plan() -> InstallPlan {
    InstallPlan {
        strategy: "ensure-ssh",
        program: "bash".into(),
        args: vec![
            "-lc".into(),
            "brew install openssh".into(),
        ],
        envs: Vec::new(),
    }
}

pub fn wsl_openclaw_install_plan(target_profile: &TargetProfile) -> InstallPlan {
    wsl_shell_plan(
        "official",
        target_profile,
        "curl -fsSL --proto '=https' --tlsv1.2 https://openclaw.ai/install.sh | bash -s -- --no-prompt --no-onboard",
    )
}

pub fn wsl_shell_plan(
    strategy: &'static str,
    target_profile: &TargetProfile,
    shell_payload: &str,
) -> InstallPlan {
    InstallPlan {
        strategy,
        program: "wsl.exe".into(),
        args: vec![
            "-d".into(),
            target_profile.wsl_distro.clone(),
            "--".into(),
            "bash".into(),
            "-lc".into(),
            shell_payload.into(),
        ],
        envs: Vec::new(),
    }
}

pub fn run_wsl_command(
    target_profile: &TargetProfile,
    shell_payload: &str,
) -> Result<std::process::Output> {
    Command::new("wsl.exe")
        .args([
            "-d",
            target_profile.wsl_distro.as_str(),
            "--",
            "bash",
            "-lc",
            shell_payload,
        ])
        .output()
        .map_err(Into::into)
}

fn listed_wsl_distros() -> Result<Vec<String>> {
    let output = Command::new("wsl.exe")
        .args(["-l", "-q"])
        .output()
        .or_else(|_| Command::new("wsl").args(["-l", "-q"]).output())?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn read_first_non_empty_line(output: std::process::Output) -> Option<String> {
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .map(str::to_string);
    if stdout.is_some() {
        return stdout;
    }

    String::from_utf8_lossy(&output.stderr)
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .map(str::to_string)
}

fn version_segments(input: &str) -> Vec<u32> {
    let mut segments = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            if let Ok(value) = current.parse::<u32>() {
                segments.push(value);
            }
            current.clear();
        }
    }

    if !current.is_empty() {
        if let Ok(value) = current.parse::<u32>() {
            segments.push(value);
        }
    }

    segments
}

fn apple_quote(input: &str) -> String {
    format!("\"{}\"", input.replace('\\', "\\\\").replace('"', "\\\""))
}

fn ps_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "''"))
}

fn sh_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::{
        compare_versions, default_runtime_target, fallback_install_plans,
        install_plans_for_target, looks_like_permission_error, macos_ensure_ssh_plan,
        official_install_plan, update_plans_for_target, Platform,
    };
    use crate::types::{RuntimeTarget, TargetProfile};

    #[test]
    fn macos_official_plan_uses_install_sh() {
        let plan = official_install_plan(Platform::MacOs);

        assert_eq!(plan.strategy, "official");
        assert_eq!(plan.program, "bash");
        let command = plan.args.join(" ");
        assert!(command.contains("install.sh"));
        assert!(command.contains("--proto '=https'"));
        assert!(command.contains("--tlsv1.2"));
        assert!(command.contains("--no-prompt"));
        assert!(command.contains("--no-onboard"));
    }

    #[test]
    fn windows_official_plan_uses_wsl_bootstrap() {
        let plan = official_install_plan(Platform::Windows);

        assert_eq!(plan.strategy, "bootstrap-wsl");
        assert_eq!(plan.program, "wsl.exe");
        assert!(plan.args.join(" ").contains("--install"));
    }

    #[test]
    fn fallback_prefers_npm_then_pnpm() {
        let plans = fallback_install_plans(true, true);
        let strategies = plans.iter().map(|plan| plan.strategy).collect::<Vec<_>>();

        assert_eq!(strategies, vec!["npm", "pnpm"]);
    }

    #[test]
    fn macos_ensure_ssh_plan_uses_homebrew() {
        let plan = macos_ensure_ssh_plan();

        assert_eq!(plan.strategy, "ensure-ssh");
        assert_eq!(plan.program, "bash");
        assert!(plan.args.join(" ").contains("brew install openssh"));
    }

    #[test]
    fn install_plans_keep_official_installer_first() {
        let plans = install_plans_for_target(
            RuntimeTarget::MacNative,
            &TargetProfile::default(),
            true,
            true,
            true,
        );
        let strategies = plans.iter().map(|plan| plan.strategy).collect::<Vec<_>>();

        assert_eq!(strategies, vec!["official", "npm", "pnpm"]);
    }

    #[test]
    fn update_plans_keep_official_installer_first() {
        let plans = update_plans_for_target(
            RuntimeTarget::MacNative,
            &TargetProfile::default(),
            true,
            true,
            true,
        );
        let strategies = plans.iter().map(|plan| plan.strategy).collect::<Vec<_>>();

        assert_eq!(strategies, vec!["official", "npm", "pnpm"]);
    }

    #[test]
    fn detects_permission_related_failures() {
        let output = "Permission denied while writing to /usr/local/bin";
        assert!(looks_like_permission_error(output));
    }

    #[test]
    fn chooses_runtime_target_from_platform() {
        assert_eq!(
            default_runtime_target(Platform::MacOs),
            RuntimeTarget::MacNative
        );
        assert_eq!(
            default_runtime_target(Platform::Windows),
            RuntimeTarget::WindowsWsl
        );
    }

    #[test]
    fn compares_openclaw_versions_using_numeric_segments() {
        assert!(compare_versions("OpenClaw 2026.3.8", "2026.3.9"));
        assert!(!compare_versions("2026.3.9", "OpenClaw 2026.3.8"));
        assert!(!compare_versions("2026.3.9", "2026.3.9"));
    }
}
