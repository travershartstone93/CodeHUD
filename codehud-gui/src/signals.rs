use crate::{GuiMessage, GuiResult, GuiError};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type SignalCallback = Box<dyn Fn(&GuiMessage) -> GuiResult<()> + Send + Sync>;

pub struct SignalBus {
    sender: Sender<GuiMessage>,
    receiver: Receiver<GuiMessage>,
    connections: Arc<Mutex<HashMap<String, Vec<SignalCallback>>>>,
}

impl SignalBus {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            sender,
            receiver,
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn emit(&self, signal_name: &str, message: GuiMessage) -> GuiResult<()> {
        self.sender.send(message)
            .map_err(|e| GuiError::State(format!("Failed to emit signal '{}': {}", signal_name, e)))?;
        Ok(())
    }

    pub fn connect(&self, signal_name: &str, callback: SignalCallback) -> GuiResult<()> {
        let mut connections = self.connections.lock()
            .map_err(|e| GuiError::State(format!("Failed to lock connections: {}", e)))?;

        connections
            .entry(signal_name.to_string())
            .or_insert_with(Vec::new)
            .push(callback);

        Ok(())
    }

    pub fn disconnect(&self, signal_name: &str) -> GuiResult<()> {
        let mut connections = self.connections.lock()
            .map_err(|e| GuiError::State(format!("Failed to lock connections: {}", e)))?;

        connections.remove(signal_name);
        Ok(())
    }

    pub fn process_pending(&self) -> GuiResult<()> {
        while let Ok(message) = self.receiver.try_recv() {
            self.dispatch_message(&message)?;
        }
        Ok(())
    }

    fn dispatch_message(&self, message: &GuiMessage) -> GuiResult<()> {
        let signal_name = self.message_to_signal_name(message);

        let connections = self.connections.lock()
            .map_err(|e| GuiError::State(format!("Failed to lock connections: {}", e)))?;

        if let Some(callbacks) = connections.get(&signal_name) {
            for callback in callbacks {
                if let Err(e) = callback(message) {
                    log::error!("Signal callback error for '{}': {}", signal_name, e);
                }
            }
        }

        Ok(())
    }

    fn message_to_signal_name(&self, message: &GuiMessage) -> String {
        match message {
            GuiMessage::ProjectLoaded(_) => "project_loaded".to_string(),
            GuiMessage::AnalysisComplete => "analysis_complete".to_string(),
            GuiMessage::LlmRequest(_) => "llm_request".to_string(),
            GuiMessage::LlmResponse(_) => "llm_response".to_string(),
            GuiMessage::QualityUpdate => "quality_updated".to_string(),
            GuiMessage::TopologyUpdate => "topology_updated".to_string(),
            GuiMessage::HealthUpdate => "health_updated".to_string(),
            GuiMessage::Error(_) => "error_occurred".to_string(),
        }
    }
}

impl Default for SignalBus {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! connect_signal {
    ($bus:expr, $signal:expr, $handler:expr) => {
        $bus.connect($signal, Box::new($handler))?;
    };
}

#[macro_export]
macro_rules! emit_signal {
    ($bus:expr, $signal:expr, $message:expr) => {
        $bus.emit($signal, $message)?;
    };
}