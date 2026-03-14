use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use tauri::{AppHandle, Emitter};

use crate::{
    app_menu,
    runtime::{
        build_native_openclaw_command, build_native_ssh_command, build_wsl_openclaw_command,
        build_wsl_ssh_command, CommandSpec,
    },
    runtime_state::RuntimeModel,
    types::{
        CompanyProfile, LocalePreference, LogEntry, RuntimeStatus, RuntimeTarget, TargetProfile,
        UserProfile,
    },
};

const LOG_LIMIT: usize = 400;
const SSH_BOOT_WAIT: Duration = Duration::from_secs(2);
const SSH_TUNNEL_READY_TIMEOUT: Duration = Duration::from_secs(6);
const SSH_TUNNEL_POLL_INTERVAL: Duration = Duration::from_millis(150);
const DEFAULT_NODE_BOOT_WAIT: Duration = Duration::from_secs(1);
const WINDOWS_NODE_BOOT_WAIT: Duration = Duration::from_secs(4);

fn locale_text(locale: LocalePreference, zh: &'static str, en: &'static str) -> &'static str {
    if matches!(locale, LocalePreference::EnUs) {
        en
    } else {
        zh
    }
}

fn locale_owned(locale: LocalePreference, zh: impl Into<String>, en: impl Into<String>) -> String {
    if matches!(locale, LocalePreference::EnUs) {
        en.into()
    } else {
        zh.into()
    }
}

#[derive(Default)]
pub struct RuntimeSupervisor {
    model: RuntimeModel,
    logs: VecDeque<LogEntry>,
    ssh_child: Option<Child>,
    node_child: Option<Child>,
}

pub type SharedRuntimeState = Arc<Mutex<RuntimeSupervisor>>;

pub fn new_shared_runtime_state() -> SharedRuntimeState {
    Arc::new(Mutex::new(RuntimeSupervisor::default()))
}

pub fn snapshot_status(state: &SharedRuntimeState) -> RuntimeStatus {
    let guard = state.lock().expect("runtime mutex poisoned");
    guard.model.status()
}

pub fn snapshot_logs(state: &SharedRuntimeState) -> Vec<LogEntry> {
    let guard = state.lock().expect("runtime mutex poisoned");
    guard.logs.iter().cloned().collect()
}

pub fn mark_configured(state: &SharedRuntimeState, app: &AppHandle) {
    {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.model.mark_configured();
    }
    emit_status(state, app);
}

pub fn start_runtime_processes(
    state: SharedRuntimeState,
    app: AppHandle,
    locale: LocalePreference,
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    company_profile: &CompanyProfile,
    user_profile: &UserProfile,
    token: &str,
    ssh_password: Option<&str>,
) -> Result<RuntimeStatus> {
    stop_runtime_processes(state.clone(), app.clone(), locale).ok();

    {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.logs.clear();
        guard.model.mark_connecting();
    }
    emit_status(&state, &app);
    append_log(
        &state,
        &app,
        "system",
        "info",
        locale_owned(
            locale,
            format!("准备建立到 {} 的 SSH 隧道", company_profile.ssh_host),
            format!("Preparing an SSH tunnel to {}", company_profile.ssh_host),
        ),
    );

    let result: Result<RuntimeStatus> = (|| {
        let ssh_command = build_runtime_ssh_command(
            locale,
            runtime_target.clone(),
            target_profile,
            company_profile,
            ssh_password,
        )?;
        let mut ssh_child = spawn_command(&ssh_command, locale).context(locale_text(
            locale,
            "无法启动 SSH 隧道",
            "Failed to start the SSH tunnel",
        ))?;
        attach_child_logs(
            &app,
            &state,
            "ssh",
            ssh_child.stdout.take(),
            ssh_child.stderr.take(),
        );
        {
            let mut guard = state.lock().expect("runtime mutex poisoned");
            guard.ssh_child = Some(ssh_child);
        }

        thread::sleep(SSH_BOOT_WAIT);
        ensure_process_alive(&state, &app, locale, ManagedProcess::Ssh)?;
        wait_for_ssh_tunnel(&state, &app, locale, company_profile.local_port)?;

        append_log(
            &state,
            &app,
            "system",
            "info",
            locale_text(
                locale,
                "SSH 隧道已建立，正在启动 OpenClaw Node",
                "SSH tunnel established. Starting OpenClaw Node",
            ),
        );
        let mut node_child = spawn_command(
            &build_runtime_openclaw_command(
                runtime_target.clone(),
                target_profile,
                company_profile,
                user_profile,
                token,
            ),
            locale,
        )
        .context(locale_text(
            locale,
            "无法启动 OpenClaw Node",
            "Failed to start OpenClaw Node",
        ))?;
        attach_child_logs(
            &app,
            &state,
            "openclaw",
            node_child.stdout.take(),
            node_child.stderr.take(),
        );
        {
            let mut guard = state.lock().expect("runtime mutex poisoned");
            guard.node_child = Some(node_child);
        }

        wait_for_node_process(&state, &app, locale, runtime_target)?;

        {
            let mut guard = state.lock().expect("runtime mutex poisoned");
            guard.model.mark_running();
        }
        emit_status(&state, &app);
        append_log(
            &state,
            &app,
            "system",
            "info",
            locale_text(
                locale,
                "OpenClaw Node 已连接到公司网关。",
                "OpenClaw Node is connected to the company gateway.",
            ),
        );

        Ok(snapshot_status(&state))
    })();

    if let Err(error) = &result {
        let needs_state_update = !matches!(
            snapshot_status(&state).phase,
            crate::types::RuntimePhase::Error
        );
        cleanup_processes(&state, locale);
        if needs_state_update {
            mark_runtime_error(&state, &app, &error.to_string());
        }
    }

    result
}

fn build_runtime_ssh_command(
    locale: LocalePreference,
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    company_profile: &CompanyProfile,
    ssh_password: Option<&str>,
) -> Result<CommandSpec> {
    match runtime_target {
        RuntimeTarget::MacNative => {
            if let Some(password) = ssh_password {
                let askpass_program = std::env::current_exe().context(locale_text(
                    locale,
                    "无法定位 SSH 密码辅助程序",
                    "Failed to locate the SSH password helper",
                ))?;
                return Ok(build_native_ssh_command(
                    company_profile,
                    Some((askpass_program.to_string_lossy().as_ref(), password)),
                ));
            }

            Ok(build_native_ssh_command(company_profile, None))
        }
        RuntimeTarget::WindowsWsl => Ok(build_wsl_ssh_command(
            target_profile,
            company_profile,
            ssh_password,
        )),
    }
}

fn build_runtime_openclaw_command(
    runtime_target: RuntimeTarget,
    target_profile: &TargetProfile,
    company_profile: &CompanyProfile,
    user_profile: &UserProfile,
    token: &str,
) -> CommandSpec {
    match runtime_target {
        RuntimeTarget::MacNative => {
            build_native_openclaw_command(company_profile, user_profile, token)
        }
        RuntimeTarget::WindowsWsl => {
            build_wsl_openclaw_command(target_profile, company_profile, user_profile, token)
        }
    }
}

pub fn stop_runtime_processes(
    state: SharedRuntimeState,
    app: AppHandle,
    locale: LocalePreference,
) -> Result<RuntimeStatus> {
    let mut node_child = {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.node_child.take()
    };
    if let Some(child) = node_child.as_mut() {
        kill_child(
            child,
            locale,
            locale_text(locale, "OpenClaw Node", "OpenClaw Node"),
        )?;
        append_log(
            &state,
            &app,
            "system",
            "info",
            locale_text(locale, "OpenClaw Node 已停止。", "OpenClaw Node stopped."),
        );
    }

    let mut ssh_child = {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.ssh_child.take()
    };
    if let Some(child) = ssh_child.as_mut() {
        kill_child(child, locale, locale_text(locale, "SSH 隧道", "SSH tunnel"))?;
        append_log(
            &state,
            &app,
            "system",
            "info",
            locale_text(locale, "SSH 隧道已关闭。", "SSH tunnel closed."),
        );
    }

    {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.model.mark_stopped();
    }
    emit_status(&state, &app);

    Ok(snapshot_status(&state))
}

fn spawn_command(spec: &CommandSpec, locale: LocalePreference) -> Result<Child> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in &spec.envs {
        command.env(key, value);
    }
    command.spawn().with_context(|| {
        format!(
            "{}: {}",
            locale_text(locale, "启动命令失败", "Failed to start command"),
            spec.program
        )
    })
}

fn attach_child_logs(
    app: &AppHandle,
    state: &SharedRuntimeState,
    source: &'static str,
    stdout: Option<impl std::io::Read + Send + 'static>,
    stderr: Option<impl std::io::Read + Send + 'static>,
) {
    if let Some(reader) = stdout {
        spawn_log_reader(app.clone(), state.clone(), source, "info", reader);
    }

    if let Some(reader) = stderr {
        spawn_log_reader(app.clone(), state.clone(), source, "error", reader);
    }
}

fn spawn_log_reader(
    app: AppHandle,
    state: SharedRuntimeState,
    source: &'static str,
    level: &'static str,
    reader: impl std::io::Read + Send + 'static,
) {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            append_log(&state, &app, source, level, line.to_string());
        }
    });
}

fn ensure_process_alive(
    state: &SharedRuntimeState,
    app: &AppHandle,
    locale: LocalePreference,
    process: ManagedProcess,
) -> Result<()> {
    let exited = {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        process
            .child_mut(&mut guard)
            .and_then(|child| child.try_wait().ok())
            .flatten()
    };

    if let Some(status) = exited {
        let detail = locale_owned(
            locale,
            format!("{} 退出，状态码: {}", process.label(locale), status),
            format!("{} exited with status {}", process.label(locale), status),
        );
        mark_runtime_error(state, app, &detail);
        let recent = recent_logs(state);
        return Err(anyhow!("{}\n{}", detail, recent));
    }

    Ok(())
}

fn kill_child(child: &mut Child, locale: LocalePreference, label: &str) -> Result<()> {
    child.kill().with_context(|| {
        locale_owned(
            locale,
            format!("结束 {} 进程失败", label),
            format!("Failed to stop the {} process", label),
        )
    })?;
    let _ = child.wait();
    Ok(())
}

fn wait_for_ssh_tunnel(
    state: &SharedRuntimeState,
    app: &AppHandle,
    locale: LocalePreference,
    local_port: u16,
) -> Result<()> {
    let started = std::time::Instant::now();

    loop {
        if wait_for_local_port(
            local_port,
            SSH_TUNNEL_POLL_INTERVAL,
            SSH_TUNNEL_POLL_INTERVAL,
            locale,
        )
        .is_ok()
        {
            return Ok(());
        }

        ensure_process_alive(state, app, locale, ManagedProcess::Ssh)?;

        if started.elapsed() >= SSH_TUNNEL_READY_TIMEOUT {
            cleanup_processes(state, locale);
            let detail = locale_owned(
                locale,
                format!(
                    "SSH 本地端口 {} 未就绪，请检查 SSH 登录和端口转发配置",
                    local_port
                ),
                format!(
                    "Local SSH port {} is not ready. Check the SSH credentials and port forwarding settings.",
                    local_port
                ),
            );
            mark_runtime_error(state, app, &detail);
            let recent = recent_logs(state);
            return Err(anyhow!("{}\n{}", detail, recent));
        }

        thread::sleep(SSH_TUNNEL_POLL_INTERVAL);
    }
}

fn cleanup_processes(state: &SharedRuntimeState, locale: LocalePreference) {
    let (mut ssh_child, mut node_child) = {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        (guard.ssh_child.take(), guard.node_child.take())
    };

    if let Some(child) = node_child.as_mut() {
        let _ = kill_child(
            child,
            locale,
            locale_text(locale, "OpenClaw Node", "OpenClaw Node"),
        );
    }
    if let Some(child) = ssh_child.as_mut() {
        let _ = kill_child(child, locale, locale_text(locale, "SSH 隧道", "SSH tunnel"));
    }
}

fn mark_runtime_error(state: &SharedRuntimeState, app: &AppHandle, message: &str) {
    {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.model.mark_error(message.to_string());
    }
    emit_status(state, app);
    append_log(state, app, "system", "error", message.to_string());
}

fn wait_for_local_port(
    port: u16,
    max_wait: Duration,
    poll_interval: Duration,
    locale: LocalePreference,
) -> Result<()> {
    let started = std::time::Instant::now();
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    loop {
        match TcpStream::connect_timeout(&address, poll_interval) {
            Ok(_) => return Ok(()),
            Err(error) if started.elapsed() >= max_wait => {
                return Err(anyhow!(
                    "{}",
                    locale_owned(
                        locale,
                        format!("本地 SSH 隧道端口 {} 未就绪: {}", port, error),
                        format!("Local SSH tunnel port {} is not ready: {}", port, error),
                    )
                ));
            }
            Err(_) => thread::sleep(poll_interval),
        }
    }
}

fn wait_for_node_process(
    state: &SharedRuntimeState,
    app: &AppHandle,
    locale: LocalePreference,
    runtime_target: RuntimeTarget,
) -> Result<()> {
    let started = std::time::Instant::now();
    let boot_wait = node_boot_wait(runtime_target);

    loop {
        ensure_process_alive(state, app, locale, ManagedProcess::OpenClaw)?;

        if started.elapsed() >= boot_wait {
            return Ok(());
        }

        thread::sleep(SSH_TUNNEL_POLL_INTERVAL);
    }
}

fn node_boot_wait(runtime_target: RuntimeTarget) -> Duration {
    node_boot_wait_for_target(runtime_target)
}

fn node_boot_wait_for_target(runtime_target: RuntimeTarget) -> Duration {
    match runtime_target {
        RuntimeTarget::WindowsWsl => WINDOWS_NODE_BOOT_WAIT,
        RuntimeTarget::MacNative => DEFAULT_NODE_BOOT_WAIT,
    }
}

fn emit_status(state: &SharedRuntimeState, app: &AppHandle) {
    let _ = app.emit("runtime-status", snapshot_status(state));
    let _ = app_menu::refresh_status_menu_from_state(app);
}

fn append_log(
    state: &SharedRuntimeState,
    app: &AppHandle,
    source: impl Into<String>,
    level: impl Into<String>,
    message: impl Into<String>,
) {
    let entry = LogEntry {
        source: source.into(),
        level: level.into(),
        message: message.into(),
        timestamp_ms: timestamp_ms(),
    };

    {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        if guard.logs.len() >= LOG_LIMIT {
            guard.logs.pop_front();
        }
        guard.logs.push_back(entry.clone());
    }
    let _ = app.emit("runtime-log", entry);
}

fn recent_logs(state: &SharedRuntimeState) -> String {
    let guard = state.lock().expect("runtime mutex poisoned");
    guard
        .logs
        .iter()
        .rev()
        .take(8)
        .map(|entry| format!("[{}] {}", entry.source, entry.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

enum ManagedProcess {
    Ssh,
    OpenClaw,
}

impl ManagedProcess {
    fn label(&self, locale: LocalePreference) -> &'static str {
        match self {
            Self::Ssh => locale_text(locale, "SSH 隧道", "SSH tunnel"),
            Self::OpenClaw => locale_text(locale, "OpenClaw Node", "OpenClaw Node"),
        }
    }

    fn child_mut<'a>(&self, runtime: &'a mut RuntimeSupervisor) -> Option<&'a mut Child> {
        match self {
            Self::Ssh => runtime.ssh_child.as_mut(),
            Self::OpenClaw => runtime.node_child.as_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, thread, time::Duration};

    use super::{node_boot_wait_for_target, wait_for_local_port};
    use crate::types::{LocalePreference, RuntimeTarget};

    #[test]
    fn waits_until_local_port_becomes_available() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(wait_for_local_port(
            port,
            Duration::from_millis(50),
            Duration::from_millis(10),
            LocalePreference::ZhCn,
        )
        .is_ok());
    }

    #[test]
    fn times_out_when_local_port_never_opens() {
        let port = unused_local_port();

        let error = wait_for_local_port(
            port,
            Duration::from_millis(80),
            Duration::from_millis(20),
            LocalePreference::EnUs,
        )
        .unwrap_err();

        assert!(error.to_string().contains("Local SSH tunnel port"));
    }

    #[test]
    fn windows_node_boot_wait_is_longer_than_default() {
        assert_eq!(
            node_boot_wait_for_target(RuntimeTarget::WindowsWsl),
            Duration::from_secs(4)
        );
        assert_eq!(
            node_boot_wait_for_target(RuntimeTarget::MacNative),
            Duration::from_secs(1)
        );
    }

    fn unused_local_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        thread::sleep(Duration::from_millis(10));
        port
    }
}
