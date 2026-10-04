//! Each surface's accessibility tree on AT-SPI, through an accesskit adapter.
//!
//! The adapter calls its handlers on its own D-Bus thread; they queue requests here and wake the
//! loop, which answers them with the tree on the next dispatch.

use std::sync::{Arc, Mutex};

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};
use smithay::reexports::calloop::ping::Ping;

pub enum Request {
    Activate,
    Deactivate,
    Action(ActionRequest),
}

/// Requests from every adapter, each with its instance's id.
#[derive(Clone)]
pub struct Requests {
    queue: Arc<Mutex<Vec<(u64, Request)>>>,
    ping: Ping,
}

impl Requests {
    pub fn new(ping: Ping) -> Self {
        Self {
            queue: Default::default(),
            ping,
        }
    }

    pub fn take(&self) -> Vec<(u64, Request)> {
        std::mem::take(&mut *self.queue.lock().unwrap())
    }

    fn push(&self, id: u64, request: Request) {
        self.queue.lock().unwrap().push((id, request));
        self.ping.ping();
    }

    pub fn adapter(&self, id: u64) -> accesskit_unix::Adapter {
        let handler = Handler {
            id,
            requests: self.clone(),
        };
        accesskit_unix::Adapter::new(handler.clone(), handler.clone(), handler)
    }
}

#[derive(Clone)]
struct Handler {
    id: u64,
    requests: Requests,
}

impl ActivationHandler for Handler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.requests.push(self.id, Request::Activate);
        None
    }
}

impl ActionHandler for Handler {
    fn do_action(&mut self, request: ActionRequest) {
        self.requests.push(self.id, Request::Action(request));
    }
}

impl DeactivationHandler for Handler {
    fn deactivate_accessibility(&mut self) {
        self.requests.push(self.id, Request::Deactivate);
    }
}

/// A tree as JSON for `Shell1.Tree`: each node's id, role, label, children and focus.
pub fn to_json(update: &TreeUpdate) -> String {
    let nodes: Vec<serde_json::Value> = update
        .nodes
        .iter()
        .map(|(id, node)| {
            serde_json::json!({
                "id": id.0,
                "role": format!("{:?}", node.role()),
                "label": node.label(),
                "value": node.value(),
                "children": node.children().iter().map(|c| c.0).collect::<Vec<_>>(),
                "focused": *id == update.focus,
            })
        })
        .collect();
    serde_json::json!({ "focus": update.focus.0, "nodes": nodes }).to_string()
}
