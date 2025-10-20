//! PyQt5-Style Signal/Slot System - Zero Degradation Implementation
//!
//! This module provides an exact PyQt5 signal/slot architecture implementation
//! to ensure zero-degradation from the Python GUI system.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use crossbeam_channel::{bounded, Receiver, Sender};
use crate::{GuiResult, GuiError};

/// PyQt5-style Signal that can emit typed messages to connected slots
pub struct PyQtSignal<T: Clone + Send + 'static> {
    slots: Arc<Mutex<Vec<Box<dyn Fn(T) + Send + Sync>>>>,
    emit_queue: Option<Sender<T>>,
    _phantom: PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> PyQtSignal<T> {
    pub fn new() -> Self {
        Self {
            slots: Arc::new(Mutex::new(Vec::new())),
            emit_queue: None,
            _phantom: PhantomData,
        }
    }

    /// Connect a slot function to this signal (PyQt5 style)
    pub fn connect<F>(&self, slot: F) -> GuiResult<()>
    where
        F: Fn(T) + Send + Sync + 'static,
    {
        let mut slots = self.slots.lock()
            .map_err(|e| GuiError::State(format!("Failed to lock signal slots: {}", e)))?;
        slots.push(Box::new(slot));
        Ok(())
    }

    /// Emit signal with value (PyQt5 style)
    pub fn emit(&self, value: T) -> GuiResult<()> {
        let slots = self.slots.lock()
            .map_err(|e| GuiError::State(format!("Failed to lock signal slots: {}", e)))?;

        for slot in slots.iter() {
            slot(value.clone());
        }

        // Also send to emit queue if connected
        if let Some(ref sender) = self.emit_queue {
            sender.try_send(value)
                .map_err(|e| GuiError::State(format!("Failed to send to emit queue: {}", e)))?;
        }

        Ok(())
    }

    /// Disconnect all slots
    pub fn disconnect_all(&self) -> GuiResult<()> {
        let mut slots = self.slots.lock()
            .map_err(|e| GuiError::State(format!("Failed to lock signal slots: {}", e)))?;
        slots.clear();
        Ok(())
    }

    /// Set up emit queue for async processing
    pub fn setup_emit_queue(&mut self, capacity: usize) -> Receiver<T> {
        let (sender, receiver) = bounded(capacity);
        self.emit_queue = Some(sender);
        receiver
    }

    /// Clone signal for thread sharing
    pub fn clone(&self) -> Self {
        Self {
            slots: Arc::clone(&self.slots),
            emit_queue: self.emit_queue.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Default for PyQtSignal<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// PyQt5-style QObject equivalent with signal/slot management
pub trait PyQtObject {
    fn setup_signals(&mut self) -> GuiResult<()>;
    fn connect_signals(&self) -> GuiResult<()>;
    fn disconnect_signals(&self) -> GuiResult<()>;
}

/// Signal manager matching PyQt5 QObject behavior
pub struct SignalManager {
    signal_connections: HashMap<String, Box<dyn Any + Send + Sync>>,
    weak_refs: Vec<Weak<dyn Any + Send + Sync>>,
}

impl SignalManager {
    pub fn new() -> Self {
        Self {
            signal_connections: HashMap::new(),
            weak_refs: Vec::new(),
        }
    }

    /// Register a signal by name (PyQt5 style)
    pub fn register_signal<T: Clone + Send + Sync + 'static>(&mut self, name: &str, signal: PyQtSignal<T>) {
        self.signal_connections.insert(name.to_string(), Box::new(signal));
    }

    /// Get signal by name and type
    pub fn get_signal<T: Clone + Send + Sync + 'static>(&self, name: &str) -> Option<&PyQtSignal<T>> {
        self.signal_connections.get(name)
            .and_then(|signal| signal.downcast_ref::<PyQtSignal<T>>())
    }

    /// Connect two objects' signals and slots (PyQt5 style)
    pub fn connect_objects<T: Clone + Send + Sync + 'static>(
        &self,
        sender_signal_name: &str,
        receiver_slot: Box<dyn Fn(T) + Send + Sync>,
    ) -> GuiResult<()> {
        if let Some(signal) = self.get_signal::<T>(sender_signal_name) {
            signal.connect(receiver_slot)?;
        } else {
            return Err(GuiError::State(format!("Signal '{}' not found", sender_signal_name)));
        }
        Ok(())
    }

    /// Cleanup disconnected signals
    pub fn cleanup(&mut self) {
        self.weak_refs.retain(|weak_ref| weak_ref.strong_count() > 0);
    }
}

impl Default for SignalManager {
    fn default() -> Self {
        Self::new()
    }
}

/// PyQt5 QThread equivalent for background processing
pub struct PyQtThread {
    name: String,
    handle: Option<std::thread::JoinHandle<()>>,
    should_stop: Arc<std::sync::atomic::AtomicBool>,

    // PyQt5-style signals
    pub started: PyQtSignal<()>,
    pub finished: PyQtSignal<()>,
    pub error: PyQtSignal<String>,
}

impl PyQtThread {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            handle: None,
            should_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            started: PyQtSignal::new(),
            finished: PyQtSignal::new(),
            error: PyQtSignal::new(),
        }
    }

    /// Start the thread (PyQt5 QThread.start() equivalent)
    pub fn start<F>(&mut self, work: F) -> GuiResult<()>
    where
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        let should_stop = self.should_stop.clone();
        let started_signal = self.started.slots.clone();
        let finished_signal = self.finished.slots.clone();
        let error_signal = self.error.slots.clone();

        let handle = std::thread::spawn(move || {
            // Emit started signal
            if let Ok(slots) = started_signal.lock() {
                for slot in slots.iter() {
                    slot(());
                }
            }

            // Run work
            let result = work();

            // Emit appropriate completion signal
            match result {
                Ok(()) => {
                    if let Ok(slots) = finished_signal.lock() {
                        for slot in slots.iter() {
                            slot(());
                        }
                    }
                }
                Err(error) => {
                    if let Ok(slots) = error_signal.lock() {
                        for slot in slots.iter() {
                            slot(error.clone());
                        }
                    }
                }
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    /// Stop the thread (PyQt5 QThread.quit() equivalent)
    pub fn quit(&self) {
        self.should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Wait for thread to finish (PyQt5 QThread.wait() equivalent)
    pub fn wait(&mut self) -> GuiResult<()> {
        if let Some(handle) = self.handle.take() {
            handle.join()
                .map_err(|e| GuiError::State(format!("Thread join error: {:?}", e)))?;
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.handle.is_some() && !self.should_stop.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Terminate the thread forcefully (PyQt5 QThread.terminate() equivalent)
    pub fn terminate(&mut self) {
        self.should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            // Note: Rust doesn't have forced thread termination for safety reasons
            // This is a graceful shutdown attempt
            let _ = handle.join();
        }
    }
}

/// Macro to create PyQt5-style signal connections
#[macro_export]
macro_rules! pyqt_connect {
    ($signal:expr, $slot:expr) => {
        $signal.connect($slot)?;
    };
}

/// Macro to emit PyQt5-style signals
#[macro_export]
macro_rules! pyqt_emit {
    ($signal:expr, $value:expr) => {
        $signal.emit($value)?;
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_slot_basic() {
        let signal = PyQtSignal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        signal.connect(move |value| {
            received_clone.lock().unwrap().push(value);
        }).unwrap();

        signal.emit(42).unwrap();
        signal.emit(84).unwrap();

        let values = received.lock().unwrap();
        assert_eq!(*values, vec![42, 84]);
    }

    #[test]
    fn test_thread_signals() {
        let mut thread = PyQtThread::new("test");
        let started = Arc::new(Mutex::new(false));
        let finished = Arc::new(Mutex::new(false));

        let started_clone = started.clone();
        let finished_clone = finished.clone();

        thread.started.connect(move |_| {
            *started_clone.lock().unwrap() = true;
        }).unwrap();

        thread.finished.connect(move |_| {
            *finished_clone.lock().unwrap() = true;
        }).unwrap();

        thread.start(|| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(())
        }).unwrap();

        thread.wait().unwrap();

        assert!(*started.lock().unwrap());
        assert!(*finished.lock().unwrap());
    }
}