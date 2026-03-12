use std::env;

use anyhow::{anyhow, Result};

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
}

pub const MANUAL_INSTALL_URL: &str = "https://docs.openclaw.ai/install";

pub fn current_platform() -> Result<Platform> {
    match env::consts::OS {
        "macos" => Ok(Platform::MacOs),
        "windows" => Ok(Platform::Windows),
        other => Err(anyhow!("暂不支持的系统: {other}")),
    }
}

pub fn command_available(name: &str) -> bool {
    which::which(name).is_ok()
}

pub fn official_install_plan(platform: Platform) -> InstallPlan {
    match platform {
        Platform::MacOs => InstallPlan {
            strategy: "official",
            program: "bash".into(),
            args: vec![
                "-lc".into(),
                "curl -fsSL https://openclaw.ai/install.sh | bash -s -- --no-onboard".into(),
            ],
        },
        Platform::Windows => InstallPlan {
            strategy: "official",
            program: "powershell".into(),
            args: vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-Command".into(),
                "& ([scriptblock]::Create((iwr -useb https://openclaw.ai/install.ps1))) -NoOnboard"
                    .into(),
            ],
        },
    }
}

pub fn fallback_install_plans(has_npm: bool, has_pnpm: bool) -> Vec<InstallPlan> {
    let mut plans = Vec::new();

    if has_npm {
        plans.push(InstallPlan {
            strategy: "npm",
            program: "npm".into(),
            args: vec!["install".into(), "-g".into(), "openclaw@latest".into()],
        });
    }

    if has_pnpm {
        plans.push(InstallPlan {
            strategy: "pnpm",
            program: "pnpm".into(),
            args: vec!["add".into(), "-g".into(), "openclaw@latest".into()],
        });
    }

    plans
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
        }),
    }
}

pub fn looks_like_permission_error(output: &str) -> bool {
    let normalized = output.to_lowercase();
    normalized.contains("permission denied")
        || normalized.contains("requires elevation")
        || normalized.contains("administrator privileges")
        || normalized.contains("access is denied")
}

fn apple_quote(input: &str) -> String {
    format!("\"{}\"", input.replace('\\', "\\\\").replace('"', "\\\""))
}

fn ps_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::{fallback_install_plans, looks_like_permission_error, official_install_plan, Platform};

    #[test]
    fn macos_official_plan_uses_install_sh() {
        let plan = official_install_plan(Platform::MacOs);

        assert_eq!(plan.strategy, "official");
        assert_eq!(plan.program, "bash");
        assert!(plan.args.join(" ").contains("install.sh"));
    }

    #[test]
    fn windows_official_plan_uses_install_ps1() {
        let plan = official_install_plan(Platform::Windows);

        assert_eq!(plan.strategy, "official");
        assert_eq!(plan.program, "powershell");
        assert!(plan.args.join(" ").contains("install.ps1"));
    }

    #[test]
    fn fallback_prefers_npm_then_pnpm() {
        let plans = fallback_install_plans(true, true);
        let strategies = plans.iter().map(|plan| plan.strategy).collect::<Vec<_>>();

        assert_eq!(strategies, vec!["npm", "pnpm"]);
    }

    #[test]
    fn detects_permission_related_failures() {
        let output = "Permission denied while writing to /usr/local/bin";
        assert!(looks_like_permission_error(output));
    }
}
