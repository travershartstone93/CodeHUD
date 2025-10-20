use anyhow::Result;
use egui::{Context, Ui};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod app;
pub mod components;
pub mod controllers;
pub mod views;
pub mod widgets;
pub mod signals;
pub mod signals_pyqt5;
pub mod state;
pub mod utils;

pub use app::CodeHudGuiApp;
pub use state::AppState;
pub use utils::{GuiView, GuiComponent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuiMessage {
    ProjectLoaded(String),
    AnalysisComplete,
    LlmRequest(String),
    LlmResponse(String),
    QualityUpdate,
    TopologyUpdate,
    HealthUpdate,
    Error(String),
}

pub type GuiResult<T> = Result<T, GuiError>;

#[derive(Debug, thiserror::Error)]
pub enum GuiError {
    #[error("Core engine error: {0}")]
    Core(#[from] codehud_core::Error),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("UI error: {0}")]
    Ui(String),

    #[error("State error: {0}")]
    State(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// Traits moved to utils.rs to avoid duplication