use std::{
    env,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};

use crate::{
    process_exec::new_command,
    types::{RuntimeTarget, TargetProfile, WslStatus},
};

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
pub const WINDOWS_NODE_MIN_MAJOR: u32 = 22;
const WINDOWS_INSTALL_VERIFICATION_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsInstallVerification {
    Verified { path: PathBuf, version: String },
    MissingExecutable,
    CommandFailed { path: PathBuf, details: String },
    VersionTooLow {
        path: PathBuf,
        version: String,
        minimum_major: u32,
    },
}

pub fn current_platform() -> Result<Platform> {
    match env::consts::OS {
        "macos" => Ok(Platform::MacOs),
        "windows" => Ok(Platform::Windows),
        other => Err(anyhow!("暂不支持的系统: {other}")),
    }
}

pub fn default_install_target(platform: Platform) -> RuntimeTarget {
    match platform {
        Platform::MacOs => RuntimeTarget::MacNative,
        Platform::Windows => RuntimeTarget::WindowsNative,
    }
}

pub fn preferred_windows_runtime_target(
    host_openclaw_installed: bool,
    wsl_openclaw_installed: bool,
) -> RuntimeTarget {
    if host_openclaw_installed {
        RuntimeTarget::WindowsNative
    } else if wsl_openclaw_installed {
        RuntimeTarget::WindowsWsl
    } else {
        RuntimeTarget::WindowsNative
    }
}

pub fn resolve_runtime_target(platform: Platform, target_profile: &TargetProfile) -> RuntimeTarget {
    match platform {
        Platform::MacOs => RuntimeTarget::MacNative,
        Platform::Windows => {
            let host_openclaw_installed = windows_local_openclaw_ready();
            if host_openclaw_installed {
                RuntimeTarget::WindowsNative
            } else {
                preferred_windows_runtime_target(
                    host_openclaw_installed,
                    target_command_available(RuntimeTarget::WindowsWsl, target_profile, "openclaw"),
                )
            }
        }
    }
}

pub fn command_available(name: &str) -> bool {
    which::which(name).is_ok()
}

pub fn windows_local_ssh_ready() -> bool {
    windows_ssh_executable_path().is_some()
}

pub fn windows_local_node_ready() -> bool {
    windows_local_node_version()
        .as_deref()
        .map(node_version_satisfies_minimum)
        .unwrap_or(false)
}

pub fn windows_local_node_version() -> Option<String> {
    let node_program = windows_node_executable_path()?;
    let output = new_command(&node_program.to_string_lossy(), &["--version".into()])
        .output()
        .ok()?;
    read_first_non_empty_line(output)
}

pub fn windows_local_git_ready() -> bool {
    windows_local_git_version().is_some()
}

pub fn windows_local_git_version() -> Option<String> {
    let git_program = windows_git_executable_path()?;
    let output = new_command(&git_program.to_string_lossy(), &["--version".into()])
        .output()
        .ok()?;
    read_first_non_empty_line(output)
}

pub fn verify_windows_node_installation(timeout: Duration) -> WindowsInstallVerification {
    verify_windows_installation(timeout, windows_node_executable_candidates, "--version", |path, version| {
        if node_version_satisfies_minimum(version) {
            WindowsInstallVerification::Verified {
                path: path.to_path_buf(),
                version: version.to_string(),
            }
        } else {
            WindowsInstallVerification::VersionTooLow {
                path: path.to_path_buf(),
                version: version.to_string(),
                minimum_major: WINDOWS_NODE_MIN_MAJOR,
            }
        }
    })
}

pub fn verify_windows_git_installation(timeout: Duration) -> WindowsInstallVerification {
    verify_windows_installation(timeout, windows_git_executable_candidates, "--version", |path, version| {
        WindowsInstallVerification::Verified {
            path: path.to_path_buf(),
            version: version.to_string(),
        }
    })
}

pub fn verify_windows_ssh_installation(timeout: Duration) -> WindowsInstallVerification {
    verify_windows_installation(timeout, windows_ssh_executable_candidates, "-V", |path, version| {
        WindowsInstallVerification::Verified {
            path: path.to_path_buf(),
            version: version.to_string(),
        }
    })
}

pub fn windows_local_openclaw_ready() -> bool {
    windows_local_openclaw_version().is_some()
}

pub fn windows_local_openclaw_version() -> Option<String> {
    let openclaw_program = windows_openclaw_executable_path()?;
    let output = new_command(&openclaw_program.to_string_lossy(), &["--version".into()])
        .output()
        .ok()?;
    read_first_non_empty_line(output)
}

pub fn target_command_available(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
    name: &str,
) -> bool {
    match target {
        RuntimeTarget::MacNative => command_available(name),
        RuntimeTarget::WindowsNative => {
            if name.eq_ignore_ascii_case("openclaw") {
                windows_local_openclaw_ready()
            } else {
                command_available(name)
            }
        }
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
        Platform::Windows => windows_native_official_install_plan(),
    }
}

pub fn windows_native_official_install_plan() -> InstallPlan {
    InstallPlan {
        strategy: "official",
        program: "powershell".into(),
        args: vec![
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-Command".into(),
            "iwr -useb https://openclaw.ai/install.ps1 | iex".into(),
        ],
        envs: windows_native_openclaw_install_env(),
    }
}

pub fn windows_native_ensure_ssh_plan() -> InstallPlan {
    InstallPlan {
        strategy: "ensure-ssh-download",
        program: "powershell".into(),
        args: vec![
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-Command".into(),
            concat!(
                "$ErrorActionPreference='Stop'; ",
                "$capability = Get-WindowsCapability -Online | Where-Object { $_.Name -like 'OpenSSH.Client*' } | Select-Object -First 1; ",
                "if ($null -ne $capability) { ",
                "  Write-Output ('OpenSSH capability execution started: ' + $capability.Name); ",
                "  if ($capability.State -ne 'Installed') { Add-WindowsCapability -Online -Name $capability.Name | Out-Null }; ",
                "  Write-Output 'OpenSSH capability execution finished with exit code: 0'; ",
                "  Write-Output 'Verifying OpenSSH installation'; ",
                "  $capabilitySsh = Join-Path $env:WINDIR 'System32\\OpenSSH\\ssh.exe'; ",
                "  if (Test-Path $capabilitySsh) { Write-Output ('OpenSSH verification pending: ' + $capabilitySsh); exit 0 } ",
                "} ",
                "$downloadDir = Join-Path $env:LOCALAPPDATA 'BizClaw\\downloads'; ",
                "$toolsDir = Join-Path $env:LOCALAPPDATA 'BizClaw\\tools'; ",
                "New-Item -ItemType Directory -Force -Path $downloadDir | Out-Null; ",
                "New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null; ",
                "$headers = @{ 'User-Agent' = 'BizClaw' }; ",
                "$release = Invoke-RestMethod -Headers $headers -Uri 'https://api.github.com/repos/PowerShell/Win32-OpenSSH/releases/latest'; ",
                "$asset = $release.assets | Where-Object { $_.name -eq 'OpenSSH-Win64.zip' } | Select-Object -First 1; ",
                "if ($null -eq $asset) { throw 'Could not resolve the latest Win32-OpenSSH Windows x64 archive.' }; ",
                "$archive = Join-Path $downloadDir $asset.name; ",
                "if (-not (Test-Path $archive)) { Invoke-WebRequest -Headers $headers -Uri $asset.browser_download_url -OutFile $archive }; ",
                "Write-Output ('OpenSSH installer download complete: ' + $archive); ",
                "$target = Join-Path $toolsDir 'OpenSSH-Win64'; ",
                "if (Test-Path $target) { Remove-Item -Recurse -Force $target }; ",
                "Write-Output ('OpenSSH installer execution started: ' + $archive); ",
                "Expand-Archive -Path $archive -DestinationPath $toolsDir -Force; ",
                "Write-Output 'OpenSSH installer execution finished with exit code: 0'; ",
                "Write-Output 'Verifying OpenSSH installation'; ",
                "$ssh = Join-Path $target 'ssh.exe'; ",
                "if (-not (Test-Path $ssh)) { throw 'OpenSSH extracted, but ssh.exe is missing.' }; ",
                "$userPath = [Environment]::GetEnvironmentVariable('Path', 'User'); ",
                "if ([string]::IsNullOrWhiteSpace($userPath)) { ",
                "  [Environment]::SetEnvironmentVariable('Path', $target, 'User') ",
                "} elseif (-not (($userPath -split ';') | Where-Object { $_ -eq $target })) { ",
                "  [Environment]::SetEnvironmentVariable('Path', ($target + ';' + $userPath), 'User') ",
                "} ",
                "Write-Output ('OpenSSH verification pending: ' + $ssh)"
            )
            .into(),
        ],
        envs: Vec::new(),
    }
}

pub fn windows_native_ensure_node_plan() -> Option<InstallPlan> {
    Some(InstallPlan {
        strategy: "ensure-node-download",
        program: "powershell".into(),
        args: vec![
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-Command".into(),
            concat!(
                "$ErrorActionPreference='Stop'; ",
                "$downloadDir = Join-Path $env:LOCALAPPDATA 'BizClaw\\downloads'; ",
                "New-Item -ItemType Directory -Force -Path $downloadDir | Out-Null; ",
                "$index = Invoke-RestMethod -Uri 'https://nodejs.org/dist/index.json'; ",
                "$release = $index | Where-Object { $_.lts -and $_.files -contains 'win-x64-msi' } | Select-Object -First 1; ",
                "if ($null -eq $release) { throw 'Could not resolve the latest Node.js LTS Windows x64 MSI.' }; ",
                "$version = $release.version; ",
                "$fileName = \"node-$version-x64.msi\"; ",
                "$installer = Join-Path $downloadDir $fileName; ",
                "if (-not (Test-Path $installer)) { Invoke-WebRequest -Uri (\"https://nodejs.org/dist/\" + $version + \"/\" + $fileName) -OutFile $installer }; ",
                "Write-Output ('Node.js installer download complete: ' + $installer); ",
                "Write-Output ('Node.js installer execution started: ' + $installer); ",
                "$process = Start-Process msiexec.exe -Wait -PassThru -ArgumentList @('/i', $installer, '/qn', '/norestart'); ",
                "$exitCode = $process.ExitCode; ",
                "Write-Output ('Node.js installer execution finished with exit code: ' + $exitCode); ",
                "if ($exitCode -ne 0) { exit $exitCode }; ",
                "Write-Output 'Verifying Node.js installation'"
            )
            .into(),
        ],
        envs: Vec::new(),
    })
}

pub fn windows_native_ensure_git_plan() -> Option<InstallPlan> {
    Some(InstallPlan {
        strategy: "ensure-git-download",
        program: "powershell".into(),
        args: vec![
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-Command".into(),
            concat!(
                "$ErrorActionPreference='Stop'; ",
                "$downloadDir = Join-Path $env:LOCALAPPDATA 'BizClaw\\downloads'; ",
                "New-Item -ItemType Directory -Force -Path $downloadDir | Out-Null; ",
                "$headers = @{ 'User-Agent' = 'BizClaw' }; ",
                "$release = Invoke-RestMethod -Headers $headers -Uri 'https://api.github.com/repos/git-for-windows/git/releases/latest'; ",
                "$asset = $release.assets | Where-Object { $_.name -match '^Git-.*-64-bit\\.exe$' } | Select-Object -First 1; ",
                "if ($null -eq $asset) { throw 'Could not resolve the latest Git for Windows 64-bit installer.' }; ",
                "$installer = Join-Path $downloadDir $asset.name; ",
                "if (-not (Test-Path $installer)) { Invoke-WebRequest -Headers $headers -Uri $asset.browser_download_url -OutFile $installer }; ",
                "Write-Output ('Git installer download complete: ' + $installer); ",
                "Write-Output ('Git installer execution started: ' + $installer); ",
                "$process = Start-Process -FilePath $installer -Wait -PassThru -ArgumentList @('/VERYSILENT', '/NORESTART', '/NOCANCEL', '/SP-'); ",
                "$exitCode = $process.ExitCode; ",
                "Write-Output ('Git installer execution finished with exit code: ' + $exitCode); ",
                "if ($exitCode -ne 0) { exit $exitCode }; ",
                "Write-Output 'Verifying Git installation'"
            )
            .into(),
        ],
        envs: Vec::new(),
    })
}

pub fn official_install_plan_for_target(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> Option<InstallPlan> {
    match target {
        RuntimeTarget::MacNative => Some(official_install_plan(Platform::MacOs)),
        RuntimeTarget::WindowsNative => Some(windows_native_official_install_plan()),
        RuntimeTarget::WindowsWsl => Some(wsl_openclaw_install_plan(target_profile)),
    }
}

pub fn install_plans_for_target(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
    prefer_official: bool,
    has_npm: bool,
    has_pnpm: bool,
) -> Vec<InstallPlan> {
    if matches!(target, RuntimeTarget::WindowsNative) {
        return if prefer_official {
            vec![windows_native_official_install_plan()]
        } else {
            fallback_install_plans_for_target(target, target_profile, has_npm, has_pnpm)
        };
    }

    let mut plans = Vec::new();
    if prefer_official {
        if let Some(plan) = official_install_plan_for_target(target.clone(), target_profile) {
            plans.push(plan);
        }
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
    if matches!(target, RuntimeTarget::WindowsNative) {
        return if prefer_official {
            vec![windows_native_official_install_plan()]
        } else {
            fallback_install_plans_for_target(target, target_profile, has_npm, has_pnpm)
        };
    }

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
        RuntimeTarget::MacNative | RuntimeTarget::WindowsNative => {
            fallback_install_plans(has_npm, has_pnpm)
        }
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
        RuntimeTarget::MacNative => {
            new_command("openclaw", &["--version".into()])
                .output()
                .ok()?
        }
        RuntimeTarget::WindowsNative => {
            let openclaw_program = windows_openclaw_executable_path()?;
            new_command(&openclaw_program.to_string_lossy(), &["--version".into()])
                .output()
                .ok()?
        }
        RuntimeTarget::WindowsWsl => run_wsl_command(target_profile, "openclaw --version").ok()?,
    })
}

pub fn read_latest_openclaw_version(
    target: RuntimeTarget,
    target_profile: &TargetProfile,
) -> Option<String> {
    match target {
        RuntimeTarget::MacNative | RuntimeTarget::WindowsNative => {
            if command_available("npm") {
                return read_first_non_empty_line(
                    new_command("npm", &["view".into(), "openclaw".into(), "version".into()])
                        .output()
                        .ok()?,
                );
            }

            if command_available("pnpm") {
                return read_first_non_empty_line(
                    new_command(
                        "pnpm",
                        &["view".into(), "openclaw".into(), "version".into()],
                    )
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

pub fn should_retry_with_elevation(command: &str, exit_code: i32, stderr: &str) -> bool {
    if exit_code == 0 {
        return false;
    }

    let normalized = stderr.to_lowercase();
    let command = command.to_ascii_lowercase();
    let contains_any =
        |patterns: &[&str]| patterns.iter().any(|pattern| normalized.contains(pattern));
    let is_git_remote_permission_denied =
        normalized.contains("permission to ") && normalized.contains(" denied");

    if contains_any(&[
        "authentication failed",
        "repository not found",
        "could not resolve host",
        "timeout",
        "not a git repository",
    ]) || is_git_remote_permission_denied
        || (command == "git" && normalized.contains("403"))
    {
        return false;
    }

    let has_permission_signal = contains_any(&[
        "eacces",
        "eperm",
        "operation not permitted",
        "permission denied",
        "access denied",
        "access is denied",
    ]);
    if !has_permission_signal {
        return false;
    }

    if command == "git" {
        return false == (normalized.contains("repository not found")
            || normalized.contains("403")
            || is_git_remote_permission_denied);
    }

    if command == "ssh" {
        return contains_any(&[
            "known_hosts",
            "identity file",
            "private key",
            "unprotected private key file",
            "bad permissions",
            "id_rsa",
            "id_ed25519",
            ".ssh\\",
            ".ssh/",
        ]);
    }

    true
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
        args: vec!["-lc".into(), "brew install openssh".into()],
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
    new_command(
        "wsl.exe",
        &[
            "-d".into(),
            target_profile.wsl_distro.clone(),
            "--".into(),
            "bash".into(),
            "-lc".into(),
            shell_payload.into(),
        ],
    )
    .output()
    .map_err(Into::into)
}

fn listed_wsl_distros() -> Result<Vec<String>> {
    let output = new_command("wsl.exe", &["-l".into(), "-q".into()])
        .output()
        .or_else(|_| new_command("wsl", &["-l".into(), "-q".into()]).output())?;
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

fn windows_node_executable_path() -> Option<PathBuf> {
    first_existing_path(windows_node_executable_candidates())
}

pub fn windows_ssh_executable_path() -> Option<PathBuf> {
    first_existing_path(windows_ssh_executable_candidates())
}

fn windows_git_executable_path() -> Option<PathBuf> {
    first_existing_path(windows_git_executable_candidates())
}

fn windows_openclaw_executable_path() -> Option<PathBuf> {
    if let Ok(path) = which::which("openclaw") {
        return Some(path);
    }

    let output = new_command(
        "powershell",
        &[
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-Command".into(),
            concat!(
                "$machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine'); ",
                "$userPath = [Environment]::GetEnvironmentVariable('Path', 'User'); ",
                "$paths = @(); ",
                "if (-not [string]::IsNullOrWhiteSpace($machinePath)) { $paths += $machinePath }; ",
                "if (-not [string]::IsNullOrWhiteSpace($userPath)) { $paths += $userPath }; ",
                "if ($paths.Count -gt 0) { $env:Path = ($paths -join ';') }; ",
                "$command = Get-Command openclaw.cmd -ErrorAction SilentlyContinue; ",
                "if ($null -eq $command) { $command = Get-Command openclaw -ErrorAction SilentlyContinue }; ",
                "if ($null -ne $command) { Write-Output $command.Source }"
            )
            .into(),
        ],
    )
    .output()
    .ok()?;

    read_first_non_empty_line(output).map(PathBuf::from)
}

fn windows_native_openclaw_install_env() -> Vec<(String, String)> {
    let current_path = env::var_os("PATH").unwrap_or_default();
    let mut paths = Vec::new();
    append_path_if_missing(
        &mut paths,
        &current_path,
        windows_ssh_executable_path().and_then(|path| path.parent().map(Path::to_path_buf)),
    );
    append_path_if_missing(
        &mut paths,
        &current_path,
        windows_node_executable_path().and_then(|path| path.parent().map(Path::to_path_buf)),
    );
    append_path_if_missing(
        &mut paths,
        &current_path,
        windows_git_executable_path().and_then(|path| path.parent().map(Path::to_path_buf)),
    );

    if paths.is_empty() {
        return Vec::new();
    }

    paths.extend(env::split_paths(&current_path));
    let joined = env::join_paths(paths);

    match joined {
        Ok(path) => vec![("PATH".into(), path.to_string_lossy().into_owned())],
        Err(_) => Vec::new(),
    }
}

fn node_version_satisfies_minimum(version: &str) -> bool {
    version_segments(version)
        .first()
        .copied()
        .map(|major| major >= WINDOWS_NODE_MIN_MAJOR)
        .unwrap_or(false)
}

fn verify_windows_installation(
    timeout: Duration,
    candidates: impl Fn() -> Vec<PathBuf>,
    version_arg: &str,
    validate: impl Fn(&Path, &str) -> WindowsInstallVerification,
) -> WindowsInstallVerification {
    poll_windows_installation(timeout, || {
        let Some(path) = first_existing_path(candidates()) else {
            return WindowsInstallVerification::MissingExecutable;
        };

        match read_command_version(&path, version_arg) {
            Ok(version) => validate(&path, &version),
            Err(details) => WindowsInstallVerification::CommandFailed { path, details },
        }
    })
}

fn poll_windows_installation(
    timeout: Duration,
    mut probe: impl FnMut() -> WindowsInstallVerification,
) -> WindowsInstallVerification {
    let started_at = Instant::now();
    let mut last_result = probe();

    loop {
        match &last_result {
            WindowsInstallVerification::Verified { .. }
            | WindowsInstallVerification::VersionTooLow { .. } => return last_result,
            WindowsInstallVerification::MissingExecutable
            | WindowsInstallVerification::CommandFailed { .. } => {
            }
        }

        if started_at.elapsed() >= timeout {
            return last_result;
        }

        let remaining = timeout.saturating_sub(started_at.elapsed());
        thread::sleep(std::cmp::min(
            WINDOWS_INSTALL_VERIFICATION_POLL_INTERVAL,
            remaining,
        ));
        last_result = probe();
    }
}

fn read_command_version(program: &Path, version_arg: &str) -> Result<String, String> {
    let output = new_command(
        &program.to_string_lossy(),
        &[version_arg.to_string()],
    )
    .output()
    .map_err(|error| error.to_string())?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(version) = stdout
            .lines()
            .find(|line| !line.trim().is_empty())
            .or_else(|| stderr.lines().find(|line| !line.trim().is_empty()))
            .map(str::trim)
            .map(str::to_string)
        {
            return Ok(version);
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let details = if stderr.is_empty() && stdout.is_empty() {
        format!("exit status {}", output.status)
    } else if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stderr}; {stdout}")
    };

    Err(details)
}

fn windows_node_executable_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_windows_candidate(
        &mut candidates,
        env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|dir| dir.join("nodejs").join("node.exe")),
    );
    push_windows_candidate(
        &mut candidates,
        env::var_os("ProgramFiles(x86)")
            .map(PathBuf::from)
            .map(|dir| dir.join("nodejs").join("node.exe")),
    );
    push_which_candidate(&mut candidates, "node");
    candidates
}

fn windows_ssh_executable_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_windows_candidate(
        &mut candidates,
        env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|dir| dir.join("OpenSSH").join("ssh.exe")),
    );
    push_windows_candidate(
        &mut candidates,
        env::var_os("WINDIR")
            .map(PathBuf::from)
            .map(|dir| dir.join("System32").join("OpenSSH").join("ssh.exe")),
    );
    push_windows_candidate(
        &mut candidates,
        env::var_os("LocalAppData").map(PathBuf::from).map(|dir| {
            dir.join("BizClaw")
                .join("tools")
                .join("OpenSSH-Win64")
                .join("ssh.exe")
        }),
    );
    push_which_candidate(&mut candidates, "ssh");
    candidates
}

fn windows_git_executable_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_windows_candidate(
        &mut candidates,
        env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|dir| dir.join("Git").join("cmd").join("git.exe")),
    );
    push_windows_candidate(
        &mut candidates,
        env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|dir| dir.join("Git").join("bin").join("git.exe")),
    );
    push_windows_candidate(
        &mut candidates,
        env::var_os("LocalAppData")
            .map(PathBuf::from)
            .map(|dir| dir.join("Programs").join("Git").join("cmd").join("git.exe")),
    );
    push_windows_candidate(
        &mut candidates,
        env::var_os("LocalAppData")
            .map(PathBuf::from)
            .map(|dir| dir.join("Programs").join("Git").join("bin").join("git.exe")),
    );
    push_which_candidate(&mut candidates, "git");
    candidates
}

fn push_windows_candidate(candidates: &mut Vec<PathBuf>, candidate: Option<PathBuf>) {
    if let Some(path) = candidate {
        push_unique_path(candidates, path);
    }
}

fn push_which_candidate(candidates: &mut Vec<PathBuf>, command: &str) {
    if let Ok(path) = which::which(command) {
        push_unique_path(candidates, path);
    }
}

fn push_unique_path(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if candidates.iter().any(|path| path == &candidate) {
        return;
    }
    candidates.push(candidate);
}

fn first_existing_path(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.is_file())
}

fn append_path_if_missing(
    paths: &mut Vec<PathBuf>,
    current_path: &std::ffi::OsStr,
    candidate: Option<PathBuf>,
) {
    let Some(candidate) = candidate else {
        return;
    };
    let candidate_normalized = candidate.to_string_lossy().to_ascii_lowercase();
    let already_present = env::split_paths(current_path).any(|entry| {
        entry
            .to_string_lossy()
            .to_ascii_lowercase()
            .eq(&candidate_normalized)
    });
    let already_queued = paths.iter().any(|entry| {
        entry
            .to_string_lossy()
            .to_ascii_lowercase()
            .eq(&candidate_normalized)
    });
    if !already_present && !already_queued {
        paths.push(candidate);
    }
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
        compare_versions, default_install_target, fallback_install_plans, install_plans_for_target,
        macos_ensure_ssh_plan, official_install_plan, poll_windows_installation,
        preferred_windows_runtime_target, should_retry_with_elevation,
        update_plans_for_target,
        windows_native_ensure_git_plan, windows_native_ensure_node_plan,
        windows_native_ensure_ssh_plan, Platform, WindowsInstallVerification,
        WINDOWS_NODE_MIN_MAJOR,
    };
    use crate::types::{RuntimeTarget, TargetProfile};
    use std::{path::PathBuf, time::Duration};

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
    fn windows_native_install_prefers_official_installer_before_package_managers() {
        let plans = install_plans_for_target(
            RuntimeTarget::WindowsNative,
            &TargetProfile::default(),
            true,
            true,
            true,
        );
        let strategies = plans.iter().map(|plan| plan.strategy).collect::<Vec<_>>();

        assert_eq!(strategies, vec!["official"]);
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
    fn windows_native_updates_use_host_package_managers_without_official_plan() {
        let plans = update_plans_for_target(
            RuntimeTarget::WindowsNative,
            &TargetProfile::default(),
            true,
            true,
            true,
        );
        let strategies = plans.iter().map(|plan| plan.strategy).collect::<Vec<_>>();

        assert_eq!(strategies, vec!["official"]);
    }

    #[test]
    fn windows_native_updates_can_fall_back_to_package_managers_when_official_is_disabled() {
        let plans = update_plans_for_target(
            RuntimeTarget::WindowsNative,
            &TargetProfile::default(),
            false,
            true,
            true,
        );
        let strategies = plans.iter().map(|plan| plan.strategy).collect::<Vec<_>>();

        assert_eq!(strategies, vec!["npm", "pnpm"]);
    }

    #[test]
    fn retries_node_when_permission_error_is_detected() {
        assert!(should_retry_with_elevation(
            "node",
            1603,
            "EPERM: operation not permitted"
        ));
    }

    #[test]
    fn does_not_retry_when_exit_code_is_zero() {
        assert!(!should_retry_with_elevation(
            "node",
            0,
            "access denied"
        ));
    }

    #[test]
    fn does_not_retry_git_for_remote_permission_failures() {
        assert!(!should_retry_with_elevation(
            "git",
            128,
            "remote: Permission to org/repo denied to user."
        ));
        assert!(!should_retry_with_elevation(
            "git",
            128,
            "fatal: unable to access 'https://example/repo.git/': The requested URL returned error: 403"
        ));
    }

    #[test]
    fn retries_ssh_only_for_local_file_permission_failures() {
        assert!(should_retry_with_elevation(
            "ssh",
            255,
            "Bad permissions on C:\\Users\\name\\.ssh\\id_rsa: Permission denied"
        ));
        assert!(!should_retry_with_elevation(
            "ssh",
            255,
            "Permission denied (publickey,password)."
        ));
    }

    #[test]
    fn chooses_install_target_from_platform() {
        assert_eq!(
            default_install_target(Platform::MacOs),
            RuntimeTarget::MacNative
        );
        assert_eq!(
            default_install_target(Platform::Windows),
            RuntimeTarget::WindowsNative
        );
    }

    #[test]
    fn prefers_windows_native_target_when_host_openclaw_exists() {
        assert_eq!(
            preferred_windows_runtime_target(true, true),
            RuntimeTarget::WindowsNative
        );
        assert_eq!(
            preferred_windows_runtime_target(false, true),
            RuntimeTarget::WindowsWsl
        );
        assert_eq!(
            preferred_windows_runtime_target(false, false),
            RuntimeTarget::WindowsNative
        );
    }

    #[test]
    fn windows_native_ensure_ssh_plan_prefers_windows_capability_and_keeps_zip_fallback() {
        let plan = windows_native_ensure_ssh_plan();

        assert_eq!(plan.strategy, "ensure-ssh-download");
        assert_eq!(plan.program, "powershell");
        let command = plan.args.join(" ");
        assert!(command.contains("Get-WindowsCapability"));
        assert!(command.contains("Add-WindowsCapability"));
        assert!(command.contains("Win32-OpenSSH/releases/latest"));
        assert!(command.contains("OpenSSH-Win64.zip"));
        assert!(command.contains("Expand-Archive"));
        assert!(command.contains("SetEnvironmentVariable"));
    }

    #[test]
    fn windows_native_ensure_node_plan_downloads_latest_node_lts_msi() {
        let plan = windows_native_ensure_node_plan().expect("node install plan");

        assert_eq!(plan.strategy, "ensure-node-download");
        assert_eq!(plan.program, "powershell");
        let command = plan.args.join(" ");
        assert!(command.contains("https://nodejs.org/dist/index.json"));
        assert!(command.contains("win-x64-msi"));
        assert!(command.contains("msiexec.exe"));
        assert!(command.contains("installer download complete"));
        assert!(command.contains("execution finished with exit code"));
        assert!(command.contains("Verifying Node.js installation"));
    }

    #[test]
    fn windows_native_ensure_git_plan_downloads_latest_git_windows_exe() {
        let plan = windows_native_ensure_git_plan().expect("git install plan");

        assert_eq!(plan.strategy, "ensure-git-download");
        assert_eq!(plan.program, "powershell");
        let command = plan.args.join(" ");
        assert!(command.contains("git-for-windows/git/releases/latest"));
        assert!(command.contains("64-bit"));
        assert!(command.contains("/VERYSILENT"));
        assert!(command.contains("/SP-"));
        assert!(command.contains("installer download complete"));
        assert!(command.contains("execution finished with exit code"));
        assert!(command.contains("Verifying Git installation"));
    }

    #[test]
    fn compares_openclaw_versions_using_numeric_segments() {
        assert!(compare_versions("OpenClaw 2026.3.8", "2026.3.9"));
        assert!(!compare_versions("2026.3.9", "OpenClaw 2026.3.8"));
        assert!(!compare_versions("2026.3.9", "2026.3.9"));
    }

    #[test]
    fn compares_node_major_versions_using_numeric_segments() {
        assert!(super::node_version_satisfies_minimum(&format!(
            "v{}.0.0",
            WINDOWS_NODE_MIN_MAJOR
        )));
        assert!(!super::node_version_satisfies_minimum(&format!(
            "v{}.9.9",
            WINDOWS_NODE_MIN_MAJOR - 1
        )));
    }

    #[test]
    fn poll_windows_installation_waits_for_delayed_success() {
        let mut attempts = 0;
        let result = poll_windows_installation(Duration::from_millis(2500), || {
            attempts += 1;
            if attempts < 3 {
                WindowsInstallVerification::MissingExecutable
            } else {
                WindowsInstallVerification::Verified {
                    path: PathBuf::from("C:\\node.exe"),
                    version: "v22.0.0".into(),
                }
            }
        });

        assert!(matches!(
            result,
            WindowsInstallVerification::Verified { version, .. } if version == "v22.0.0"
        ));
    }

    #[test]
    fn poll_windows_installation_returns_last_failure_after_timeout() {
        let result = poll_windows_installation(Duration::from_millis(0), || {
            WindowsInstallVerification::CommandFailed {
                path: PathBuf::from("C:\\Git\\cmd\\git.exe"),
                details: "access denied".into(),
            }
        });

        assert!(matches!(
            result,
            WindowsInstallVerification::CommandFailed { details, .. } if details == "access denied"
        ));
    }
}
