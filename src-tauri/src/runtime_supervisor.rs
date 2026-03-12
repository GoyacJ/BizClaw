use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use tauri::{AppHandle, Emitter};

use crate::{
    runtime::{build_openclaw_command, build_ssh_command, CommandSpec},
    runtime_state::RuntimeModel,
    types::{CompanyProfile, LogEntry, RuntimeStatus, UserProfile},
};

const LOG_LIMIT: usize = 400;
const SSH_BOOT_WAIT: Duration = Duration::from_secs(2);
const NODE_BOOT_WAIT: Duration = Duration::from_secs(1);

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
    company_profile: &CompanyProfile,
    user_profile: &UserProfile,
    token: &str,
) -> Result<RuntimeStatus> {
    stop_runtime_processes(state.clone(), app.clone()).ok();

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
        format!("准备建立到 {} 的 SSH 隧道", company_profile.ssh_host),
    );

    let mut ssh_child = spawn_command(&build_ssh_command(company_profile)).context("无法启动 SSH 隧道")?;
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
    ensure_process_alive(&state, &app, ManagedProcess::Ssh)?;

    append_log(&state, &app, "system", "info", "SSH 隧道已建立，正在启动 OpenClaw Node");
    let mut node_child =
        spawn_command(&build_openclaw_command(company_profile, user_profile, token))
            .context("无法启动 OpenClaw Node")?;
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

    thread::sleep(NODE_BOOT_WAIT);
    ensure_process_alive(&state, &app, ManagedProcess::OpenClaw)?;

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
        "OpenClaw Node 已连接到公司网关。",
    );

    Ok(snapshot_status(&state))
}

pub fn stop_runtime_processes(state: SharedRuntimeState, app: AppHandle) -> Result<RuntimeStatus> {
    let mut node_child = {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.node_child.take()
    };
    if let Some(child) = node_child.as_mut() {
        kill_child(child, "OpenClaw Node")?;
        append_log(&state, &app, "system", "info", "OpenClaw Node 已停止。");
    }

    let mut ssh_child = {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.ssh_child.take()
    };
    if let Some(child) = ssh_child.as_mut() {
        kill_child(child, "SSH 隧道")?;
        append_log(&state, &app, "system", "info", "SSH 隧道已关闭。");
    }

    {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.model.mark_stopped();
    }
    emit_status(&state, &app);

    Ok(snapshot_status(&state))
}

fn spawn_command(spec: &CommandSpec) -> Result<Child> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in &spec.envs {
        command.env(key, value);
    }
    command
        .spawn()
        .with_context(|| format!("启动命令失败: {}", spec.program))
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
        let detail = format!("{} 退出，状态码: {}", process.label(), status);
        mark_runtime_error(state, app, &detail);
        let recent = recent_logs(state);
        return Err(anyhow!("{}\n{}", detail, recent));
    }

    Ok(())
}

fn kill_child(child: &mut Child, label: &str) -> Result<()> {
    child
        .kill()
        .with_context(|| format!("结束 {} 进程失败", label))?;
    let _ = child.wait();
    Ok(())
}

fn mark_runtime_error(state: &SharedRuntimeState, app: &AppHandle, message: &str) {
    {
        let mut guard = state.lock().expect("runtime mutex poisoned");
        guard.model.mark_error(message.to_string());
    }
    emit_status(state, app);
    append_log(state, app, "system", "error", message.to_string());
}

fn emit_status(state: &SharedRuntimeState, app: &AppHandle) {
    let _ = app.emit("runtime-status", snapshot_status(state));
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
    fn label(&self) -> &'static str {
        match self {
            Self::Ssh => "SSH 隧道",
            Self::OpenClaw => "OpenClaw Node",
        }
    }

    fn child_mut<'a>(&self, runtime: &'a mut RuntimeSupervisor) -> Option<&'a mut Child> {
        match self {
            Self::Ssh => runtime.ssh_child.as_mut(),
            Self::OpenClaw => runtime.node_child.as_mut(),
        }
    }
}
