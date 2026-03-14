use std::{
    process::Child,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};

use crate::types::{
    OperationEvent, OperationKind, OperationResult, OperationStep, OperationTaskPhase,
    OperationTaskSnapshot,
};

const OPERATION_EVENT_LIMIT: usize = 200;

#[derive(Default)]
pub struct OperationSupervisor {
    snapshot: OperationTaskSnapshot,
    cancel_requested: bool,
    child: Option<Child>,
    recent_events: Vec<OperationEvent>,
}

pub type SharedOperationState = Arc<Mutex<OperationSupervisor>>;

impl Default for OperationTaskSnapshot {
    fn default() -> Self {
        Self {
            phase: OperationTaskPhase::Idle,
            kind: None,
            step: None,
            can_stop: false,
            last_result: None,
            started_at: None,
            ended_at: None,
        }
    }
}

pub fn new_shared_operation_state() -> SharedOperationState {
    Arc::new(Mutex::new(OperationSupervisor::default()))
}

pub fn snapshot_task(state: &SharedOperationState) -> OperationTaskSnapshot {
    let guard = state.lock().expect("operation mutex poisoned");
    guard.snapshot.clone()
}

pub fn snapshot_events(state: &SharedOperationState) -> Vec<OperationEvent> {
    let guard = state.lock().expect("operation mutex poisoned");
    guard.recent_events.clone()
}

pub fn start_task(
    state: &SharedOperationState,
    kind: OperationKind,
    step: OperationStep,
) -> Result<OperationTaskSnapshot> {
    let mut guard = state.lock().expect("operation mutex poisoned");
    if matches!(
        guard.snapshot.phase,
        OperationTaskPhase::Running | OperationTaskPhase::Cancelling
    ) {
        return Err(anyhow!("当前已有安装或更新任务进行中，请稍候"));
    }

    guard.cancel_requested = false;
    guard.child = None;
    guard.recent_events.clear();
    guard.snapshot = OperationTaskSnapshot {
        phase: OperationTaskPhase::Running,
        kind: Some(kind),
        step: Some(step),
        can_stop: true,
        last_result: None,
        started_at: Some(timestamp_ms()),
        ended_at: None,
    };

    Ok(guard.snapshot.clone())
}

pub fn update_step(state: &SharedOperationState, step: OperationStep) -> OperationTaskSnapshot {
    let mut guard = state.lock().expect("operation mutex poisoned");
    guard.snapshot.step = Some(step);
    guard.snapshot.clone()
}

pub fn attach_child(state: &SharedOperationState, child: Child) {
    let mut guard = state.lock().expect("operation mutex poisoned");
    guard.child = Some(child);
    guard.snapshot.can_stop = true;
}

pub fn with_child_mut<T>(
    state: &SharedOperationState,
    handler: impl FnOnce(&mut Child) -> T,
) -> Option<T> {
    let mut guard = state.lock().expect("operation mutex poisoned");
    guard.child.as_mut().map(handler)
}

pub fn clear_child(state: &SharedOperationState) {
    let mut guard = state.lock().expect("operation mutex poisoned");
    guard.child = None;
}

pub fn push_event(state: &SharedOperationState, event: OperationEvent) {
    let mut guard = state.lock().expect("operation mutex poisoned");
    guard.recent_events.push(event);
    if guard.recent_events.len() > OPERATION_EVENT_LIMIT {
        let overflow = guard.recent_events.len() - OPERATION_EVENT_LIMIT;
        guard.recent_events.drain(0..overflow);
    }
}

pub fn request_stop(state: &SharedOperationState) -> Result<OperationTaskSnapshot> {
    let mut guard = state.lock().expect("operation mutex poisoned");
    if !matches!(
        guard.snapshot.phase,
        OperationTaskPhase::Running | OperationTaskPhase::Cancelling
    ) {
        return Err(anyhow!("当前没有可停止的安装或更新任务"));
    }

    guard.cancel_requested = true;
    guard.snapshot.phase = OperationTaskPhase::Cancelling;
    guard.snapshot.can_stop = false;
    if let Some(child) = guard.child.as_mut() {
        let _ = child.kill();
    }

    Ok(guard.snapshot.clone())
}

pub fn cancel_requested(state: &SharedOperationState) -> bool {
    let guard = state.lock().expect("operation mutex poisoned");
    guard.cancel_requested
}

pub fn finish_task(
    state: &SharedOperationState,
    phase: OperationTaskPhase,
    result: OperationResult,
) -> OperationTaskSnapshot {
    let mut guard = state.lock().expect("operation mutex poisoned");
    guard.cancel_requested = false;
    guard.child = None;
    guard.snapshot.phase = phase;
    guard.snapshot.can_stop = false;
    guard.snapshot.last_result = Some(result);
    guard.snapshot.ended_at = Some(timestamp_ms());
    guard.snapshot.clone()
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::{
        process::{Command, Stdio},
        thread,
        time::Duration,
    };

    use super::{
        attach_child, cancel_requested, finish_task, new_shared_operation_state, push_event,
        request_stop, snapshot_events, start_task, with_child_mut,
    };
    use crate::types::{
        OperationEvent, OperationEventSource, OperationEventStatus, OperationKind, OperationResult,
        OperationStep, OperationTaskPhase,
    };

    #[test]
    fn transitions_from_running_to_cancelled() {
        let state = new_shared_operation_state();

        let running = start_task(&state, OperationKind::Install, OperationStep::Detect).unwrap();
        assert_eq!(running.phase, OperationTaskPhase::Running);
        assert!(running.can_stop);

        let cancelling = request_stop(&state).unwrap();
        assert_eq!(cancelling.phase, OperationTaskPhase::Cancelling);
        assert!(cancel_requested(&state));

        let final_state = finish_task(
            &state,
            OperationTaskPhase::Cancelled,
            sample_result(false, "安装已停止。"),
        );
        assert_eq!(final_state.phase, OperationTaskPhase::Cancelled);
        assert!(!final_state.can_stop);
        assert_eq!(
            final_state
                .last_result
                .as_ref()
                .map(|result| result.follow_up.as_str()),
            Some("安装已停止。"),
        );
    }

    #[test]
    fn transitions_from_running_to_success() {
        let state = new_shared_operation_state();
        start_task(&state, OperationKind::Update, OperationStep::UpdateOpenClaw).unwrap();

        let final_state = finish_task(
            &state,
            OperationTaskPhase::Success,
            sample_result(true, "OpenClaw 更新完成。"),
        );

        assert_eq!(final_state.phase, OperationTaskPhase::Success);
        assert_eq!(
            final_state
                .last_result
                .as_ref()
                .map(|result| result.success),
            Some(true),
        );
    }

    #[test]
    fn keeps_recent_events_and_clears_them_for_new_task() {
        let state = new_shared_operation_state();
        start_task(&state, OperationKind::Install, OperationStep::Detect).unwrap();
        push_event(&state, sample_event("first"));
        push_event(&state, sample_event("second"));

        let recent = snapshot_events(&state);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[1].message, "second");

        finish_task(
            &state,
            OperationTaskPhase::Success,
            sample_result(true, "OpenClaw 安装完成。"),
        );
        start_task(&state, OperationKind::Update, OperationStep::Detect).unwrap();

        assert!(snapshot_events(&state).is_empty());
    }

    #[test]
    fn request_stop_kills_attached_child() {
        let state = new_shared_operation_state();
        start_task(
            &state,
            OperationKind::Install,
            OperationStep::InstallOpenClaw,
        )
        .unwrap();
        attach_child(&state, spawn_long_running_child());

        request_stop(&state).unwrap();

        let mut exited = false;
        for _ in 0..20 {
            let status = with_child_mut(&state, |child| child.try_wait().unwrap()).flatten();
            if status.is_some() {
                exited = true;
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        assert!(exited, "attached child should exit after stop request");
    }

    fn sample_result(success: bool, follow_up: &str) -> OperationResult {
        OperationResult {
            kind: OperationKind::Install,
            strategy: "official".into(),
            success,
            step: OperationStep::InstallOpenClaw,
            stdout: String::new(),
            stderr: String::new(),
            needs_elevation: false,
            manual_url: "https://docs.openclaw.ai/install".into(),
            follow_up: follow_up.into(),
            remediation: None,
        }
    }

    fn sample_event(message: &str) -> OperationEvent {
        OperationEvent {
            kind: OperationKind::Install,
            step: OperationStep::InstallOpenClaw,
            status: OperationEventStatus::Log,
            source: OperationEventSource::Stdout,
            message: message.into(),
            timestamp_ms: 1,
        }
    }

    fn spawn_long_running_child() -> std::process::Child {
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "ping -n 30 127.0.0.1 >NUL"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn windows long-running child")
        } else {
            Command::new("sh")
                .args(["-c", "sleep 30"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn unix long-running child")
        }
    }
}
