pub mod config;
pub mod cst;
pub mod findings;
pub mod aggregate;
pub mod render;
pub mod detectors;

pub use config::NarratorConfig;
pub use cst::{FileCst, Node};
pub use findings::{Finding, FindingType};
pub use aggregate::{FileDoc, SectionDoc, aggregate_findings};
pub use render::render_markdown;
pub use detectors::DetectorRegistry;