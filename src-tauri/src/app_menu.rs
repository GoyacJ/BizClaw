use tauri::{
    menu::{Menu, MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder},
    AppHandle, Emitter, Manager, Runtime, Wry,
};

use crate::types::{EnvironmentSnapshot, OperationTaskPhase, OperationTaskSnapshot, TokenStatus};

pub const MENU_SHOW_ID: &str = "show";
pub const MENU_REFRESH_ID: &str = "refresh";
pub const MENU_QUIT_ID: &str = "quit";
const BIZCLAW_MENU_TITLE: &str = "BizClaw";
const STATUS_SUBMENU_TITLE: &str = "状态";
const MENU_STATUS_OPENCLAW_ID: &str = "status-openclaw";
const MENU_STATUS_GATEWAY_ID: &str = "status-gateway";
const MENU_STATUS_SSH_ID: &str = "status-ssh";
const MENU_STATUS_TOKEN_ID: &str = "status-token";
const TRAY_MENU_ACTIONS: [(&str, &str); 2] =
    [(MENU_SHOW_ID, "打开控制台"), (MENU_QUIT_ID, "退出")];

#[derive(Clone)]
pub struct AppMenuState<R: Runtime = Wry> {
    pub tray_menu: Menu<R>,
    status_items: StatusMenuItems<R>,
}

#[derive(Clone)]
struct StatusMenuItems<R: Runtime = Wry> {
    openclaw: MenuItem<R>,
    gateway: MenuItem<R>,
    ssh: MenuItem<R>,
    token: MenuItem<R>,
}

impl<R: Runtime> AppMenuState<R> {
    fn new(tray_menu: Menu<R>, status_items: StatusMenuItems<R>) -> Self {
        Self {
            tray_menu,
            status_items,
        }
    }
}

pub fn build_app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let bizclaw_menu = SubmenuBuilder::with_id(app, "bizclaw-submenu", BIZCLAW_MENU_TITLE)
        .text(MENU_SHOW_ID, "打开控制台")
        .text(MENU_REFRESH_ID, "重新检测")
        .separator()
        .text(MENU_QUIT_ID, "退出")
        .build()?;

    MenuBuilder::new(app).item(&bizclaw_menu).build()
}

pub fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<AppMenuState<R>> {
    let labels = status_menu_labels(None, None);
    let openclaw = build_status_menu_item(app, MENU_STATUS_OPENCLAW_ID, &labels.openclaw)?;
    let gateway = build_status_menu_item(app, MENU_STATUS_GATEWAY_ID, &labels.gateway)?;
    let ssh = build_status_menu_item(app, MENU_STATUS_SSH_ID, &labels.ssh)?;
    let token = build_status_menu_item(app, MENU_STATUS_TOKEN_ID, &labels.token)?;
    let mut status_menu_builder =
        SubmenuBuilder::with_id(app, "tray-status-submenu", STATUS_SUBMENU_TITLE);
    for item in [&openclaw, &gateway, &ssh, &token] {
        status_menu_builder = status_menu_builder.item(item);
    }
    let status_menu = status_menu_builder.build()?;

    let mut menu_builder = MenuBuilder::new(app).item(&status_menu).separator();
    for (id, text) in TRAY_MENU_ACTIONS {
        menu_builder = menu_builder.text(id, text);
    }
    let tray_menu = menu_builder.build()?;
    Ok(AppMenuState::new(
        tray_menu,
        StatusMenuItems {
            openclaw,
            gateway,
            ssh,
            token,
        },
    ))
}

pub fn emit_refresh_request(app: &AppHandle) {
    let _ = app.emit("app-refresh-requested", ());
}

pub fn refresh_status_menu(
    app: &AppHandle,
    environment: Option<&EnvironmentSnapshot>,
    operation: Option<&OperationTaskSnapshot>,
) -> tauri::Result<()> {
    let menu_state = app.state::<AppMenuState<Wry>>();
    refresh_status_menu_items(&menu_state.status_items, environment, operation)
}

fn refresh_status_menu_items<R: Runtime>(
    status_items: &StatusMenuItems<R>,
    environment: Option<&EnvironmentSnapshot>,
    operation: Option<&OperationTaskSnapshot>,
) -> tauri::Result<()> {
    let labels = status_menu_labels(environment, operation);
    status_items.openclaw.set_text(&labels.openclaw)?;
    status_items.gateway.set_text(&labels.gateway)?;
    status_items.ssh.set_text(&labels.ssh)?;
    status_items.token.set_text(&labels.token)?;
    Ok(())
}

pub fn refresh_status_menu_from_state(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<crate::commands::AppState>();
    let mut environment = state
        .environment_cache
        .lock()
        .expect("environment cache mutex poisoned")
        .clone();
    let operation = crate::operation_supervisor::snapshot_task(&state.operation);
    let runtime_status = crate::runtime_supervisor::snapshot_status(&state.runtime);
    if let Some(snapshot) = environment.as_mut() {
        snapshot.runtime_status = runtime_status;
    }
    refresh_status_menu(app, environment.as_ref(), Some(&operation))
}

fn build_status_menu_item<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    text: &str,
) -> tauri::Result<MenuItem<R>> {
    MenuItemBuilder::with_id(id, text)
        .enabled(false)
        .build(app)
}

fn status_menu_labels(
    environment: Option<&EnvironmentSnapshot>,
    operation: Option<&OperationTaskSnapshot>,
) -> StatusMenuLabels {
    StatusMenuLabels {
        openclaw: format!("OpenClaw: {}", openclaw_label(environment, operation)),
        gateway: format!("Gateway: {}", gateway_label(environment)),
        ssh: format!("SSH: {}", ssh_label(environment)),
        token: format!("Token: {}", token_label(environment)),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StatusMenuLabels {
    openclaw: String,
    gateway: String,
    ssh: String,
    token: String,
}

fn openclaw_label(
    environment: Option<&EnvironmentSnapshot>,
    operation: Option<&OperationTaskSnapshot>,
) -> String {
    if let Some(task) = operation {
        match (task.phase, task.kind) {
            (OperationTaskPhase::Running, Some(crate::types::OperationKind::Install)) => {
                return "安装中".into()
            }
            (OperationTaskPhase::Running, Some(crate::types::OperationKind::Update)) => {
                return "更新中".into()
            }
            (OperationTaskPhase::Cancelling, _) => return "停止中".into(),
            _ => {}
        }
    }

    match environment {
        Some(snapshot) if snapshot.openclaw_installed => {
            let version = snapshot
                .openclaw_version
                .clone()
                .unwrap_or_else(|| "已安装".into());
            if snapshot.update_available {
                format!("{version} (可更新)")
            } else {
                version
            }
        }
        Some(_) => "未安装".into(),
        None => "检测中".into(),
    }
}

fn gateway_label(environment: Option<&EnvironmentSnapshot>) -> &'static str {
    match environment {
        Some(snapshot)
            if snapshot.runtime_status.phase == crate::types::RuntimePhase::Connecting =>
        {
            "连接中"
        }
        Some(snapshot) if snapshot.runtime_status.gateway_connected => "已连接",
        Some(_) => "未连接",
        None => "检测中",
    }
}

fn ssh_label(environment: Option<&EnvironmentSnapshot>) -> &'static str {
    match environment {
        Some(snapshot) if snapshot.runtime_status.ssh_connected => "已连接",
        Some(snapshot) if snapshot.target_ssh_installed => "已就绪",
        Some(_) => "待补齐",
        None => "检测中",
    }
}

fn token_label(environment: Option<&EnvironmentSnapshot>) -> &'static str {
    match environment.map(|snapshot| snapshot.token_status) {
        Some(TokenStatus::Saved) => "已保存",
        Some(TokenStatus::Error) => "存储异常",
        Some(TokenStatus::Missing) => "未保存",
        None => "检测中",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        gateway_label, openclaw_label, status_menu_labels, BIZCLAW_MENU_TITLE,
        STATUS_SUBMENU_TITLE, TRAY_MENU_ACTIONS,
    };
    use crate::types::{
        EnvironmentSnapshot, OperationKind, OperationTaskPhase, OperationTaskSnapshot,
        RuntimePhase, RuntimeStatus, RuntimeTarget, TokenStatus,
    };

    #[test]
    fn renders_openclaw_menu_labels_for_install_states() {
        let running = openclaw_label(
            Some(&sample_snapshot()),
            Some(&OperationTaskSnapshot {
                phase: OperationTaskPhase::Running,
                kind: Some(OperationKind::Install),
                step: None,
                can_stop: true,
                last_result: None,
                started_at: Some(1),
                ended_at: None,
            }),
        );
        assert_eq!(running, "安装中");

        let installed = openclaw_label(Some(&sample_snapshot()), None);
        assert!(installed.contains("2026.3.8"));
    }

    #[test]
    fn renders_openclaw_menu_labels_for_update_and_cancelling_states() {
        let updating = openclaw_label(
            Some(&sample_snapshot()),
            Some(&OperationTaskSnapshot {
                phase: OperationTaskPhase::Running,
                kind: Some(OperationKind::Update),
                step: None,
                can_stop: true,
                last_result: None,
                started_at: Some(1),
                ended_at: None,
            }),
        );
        assert_eq!(updating, "更新中");

        let cancelling = openclaw_label(
            Some(&sample_snapshot()),
            Some(&OperationTaskSnapshot {
                phase: OperationTaskPhase::Cancelling,
                kind: Some(OperationKind::Install),
                step: None,
                can_stop: false,
                last_result: None,
                started_at: Some(1),
                ended_at: None,
            }),
        );
        assert_eq!(cancelling, "停止中");
    }

    #[test]
    fn renders_gateway_menu_labels_for_runtime_states() {
        let mut snapshot = sample_snapshot();
        snapshot.runtime_status.phase = RuntimePhase::Connecting;
        assert_eq!(gateway_label(Some(&snapshot)), "连接中");

        snapshot.runtime_status.phase = RuntimePhase::Running;
        snapshot.runtime_status.gateway_connected = true;
        assert_eq!(gateway_label(Some(&snapshot)), "已连接");
    }

    #[test]
    fn app_menu_only_contains_bizclaw_submenu() {
        assert_eq!(BIZCLAW_MENU_TITLE, "BizClaw");
    }

    #[test]
    fn tray_menu_contains_status_submenu_and_actions() {
        assert_eq!(STATUS_SUBMENU_TITLE, "状态");
        assert_eq!(TRAY_MENU_ACTIONS, [("show", "打开控制台"), ("quit", "退出")]);
    }

    #[test]
    fn refreshes_tray_status_entries_from_environment() {
        let labels = status_menu_labels(Some(&sample_snapshot()), None);

        assert_eq!(labels.openclaw, "OpenClaw: OpenClaw 2026.3.8 (可更新)");
        assert_eq!(labels.gateway, "Gateway: 未连接");
        assert_eq!(labels.ssh, "SSH: 已就绪");
        assert_eq!(labels.token, "Token: 已保存");
    }

    fn sample_snapshot() -> EnvironmentSnapshot {
        EnvironmentSnapshot {
            os: "macos".into(),
            runtime_target: RuntimeTarget::MacNative,
            host_ssh_installed: true,
            target_ssh_installed: true,
            openclaw_installed: true,
            openclaw_version: Some("OpenClaw 2026.3.8".into()),
            latest_openclaw_version: Some("2026.3.9".into()),
            update_available: true,
            has_saved_profile: true,
            token_status: TokenStatus::Saved,
            token_status_message: None,
            saved_settings: None,
            runtime_status: RuntimeStatus::default(),
            install_recommendation: String::new(),
            wsl_status: None,
        }
    }
}
