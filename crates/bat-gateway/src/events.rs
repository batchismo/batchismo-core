/// Event bus — tokio broadcast channel carrying AgentToGateway events.
///
/// The Gateway publishes events here; the Tauri shell subscribes and
/// forwards them to the frontend via Tauri's own event system.

use tokio::sync::broadcast;

use bat_types::ipc::AgentToGateway;

const BUS_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AgentToGateway>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BUS_CAPACITY);
        Self { sender }
    }

    /// Subscribe to receive future events.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentToGateway> {
        self.sender.subscribe()
    }

    /// Publish an event to all current subscribers.
    /// Silently drops the event if there are no subscribers.
    pub fn send(&self, event: AgentToGateway) {
        let _ = self.sender.send(event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
