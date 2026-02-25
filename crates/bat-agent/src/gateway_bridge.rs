//! Bridge for tools to communicate with the gateway via IPC.
//!
//! The agent main loop sets up a relay between this bridge and the actual pipe.
//! Tools call `request()` which blocks until the gateway responds.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use bat_types::ipc::{ProcessAction, ProcessResult};

/// A bridge that tools use to send requests to the gateway and get responses.
#[derive(Clone)]
pub struct GatewayBridge {
    /// Sender for outgoing requests. The main loop reads from the receiver.
    tx: Arc<Mutex<tokio::sync::mpsc::UnboundedSender<(String, ProcessAction, oneshot::Sender<ProcessResult>)>>>,
    counter: Arc<Mutex<u64>>,
}

/// The receiving end that the main loop uses to relay requests to the pipe.
pub struct BridgeReceiver {
    pub rx: tokio::sync::mpsc::UnboundedReceiver<(String, ProcessAction, oneshot::Sender<ProcessResult>)>,
}

/// Pending response waiters.
pub struct BridgePending {
    waiters: Arc<Mutex<HashMap<String, oneshot::Sender<ProcessResult>>>>,
}

impl BridgePending {
    pub fn new() -> Self {
        Self {
            waiters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a waiter for a request_id.
    pub fn register(&self, request_id: String, tx: oneshot::Sender<ProcessResult>) {
        self.waiters.lock().unwrap().insert(request_id, tx);
    }

    /// Deliver a response to the waiting tool.
    pub fn deliver(&self, request_id: &str, result: ProcessResult) -> bool {
        if let Some(tx) = self.waiters.lock().unwrap().remove(request_id) {
            let _ = tx.send(result);
            true
        } else {
            false
        }
    }
}

/// Create a new bridge pair.
pub fn create_bridge() -> (GatewayBridge, BridgeReceiver) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (
        GatewayBridge {
            tx: Arc::new(Mutex::new(tx)),
            counter: Arc::new(Mutex::new(0)),
        },
        BridgeReceiver { rx },
    )
}

impl GatewayBridge {
    /// Send a process request to the gateway and wait for the response.
    /// This blocks the current thread (used from sync tool execute methods).
    pub fn request(&self, action: ProcessAction) -> ProcessResult {
        let request_id = {
            let mut c = self.counter.lock().unwrap();
            *c += 1;
            format!("req-{c}")
        };

        let (resp_tx, resp_rx) = oneshot::channel();

        {
            let tx = self.tx.lock().unwrap();
            if tx.send((request_id, action, resp_tx)).is_err() {
                return ProcessResult::Error {
                    message: "Bridge channel closed".to_string(),
                };
            }
        }

        // Block waiting for the response.
        // We use block_in_place because execute() is sync but called from within tokio.
        match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(resp_rx)
        }) {
            Ok(result) => result,
            Err(_) => ProcessResult::Error {
                message: "Response channel closed".to_string(),
            },
        }
    }
}
