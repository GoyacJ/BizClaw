use crate::types::{RuntimePhase, RuntimeStatus};

#[derive(Debug, Clone, Default)]
pub struct RuntimeModel {
    status: RuntimeStatus,
}

impl RuntimeModel {
    pub fn status(&self) -> RuntimeStatus {
        self.status.clone()
    }

    pub fn mark_configured(&mut self) {
        self.status.phase = RuntimePhase::Configured;
        self.status.ssh_connected = false;
        self.status.node_connected = false;
        self.status.last_error = None;
    }

    pub fn mark_connecting(&mut self) {
        self.status.phase = RuntimePhase::Connecting;
        self.status.ssh_connected = false;
        self.status.node_connected = false;
    }

    pub fn mark_running(&mut self) {
        self.status.phase = RuntimePhase::Running;
        self.status.ssh_connected = true;
        self.status.node_connected = true;
        self.status.last_error = None;
    }

    pub fn mark_error(&mut self, message: impl Into<String>) {
        self.status.phase = RuntimePhase::Error;
        self.status.ssh_connected = false;
        self.status.node_connected = false;
        self.status.last_error = Some(message.into());
    }

    pub fn mark_stopped(&mut self) {
        self.status.phase = RuntimePhase::Configured;
        self.status.ssh_connected = false;
        self.status.node_connected = false;
        self.status.last_error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeModel;
    use crate::types::RuntimePhase;

    #[test]
    fn moves_from_configured_to_running() {
        let mut model = RuntimeModel::default();

        model.mark_configured();
        assert_eq!(model.status().phase, RuntimePhase::Configured);

        model.mark_connecting();
        assert_eq!(model.status().phase, RuntimePhase::Connecting);

        model.mark_running();
        let status = model.status();
        assert_eq!(status.phase, RuntimePhase::Running);
        assert!(status.ssh_connected);
        assert!(status.node_connected);
    }

    #[test]
    fn error_transition_records_message_and_recovers() {
        let mut model = RuntimeModel::default();

        model.mark_error("ssh tunnel failed");
        let status = model.status();
        assert_eq!(status.phase, RuntimePhase::Error);
        assert_eq!(status.last_error.as_deref(), Some("ssh tunnel failed"));

        model.mark_stopped();
        let recovered = model.status();
        assert_eq!(recovered.phase, RuntimePhase::Configured);
        assert_eq!(recovered.last_error, None);
    }
}
