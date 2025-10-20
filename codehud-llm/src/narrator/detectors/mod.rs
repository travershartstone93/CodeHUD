use crate::narrator::{FileCst, Finding, NarratorConfig};

pub mod comments;
pub mod entrypoint;
pub mod wrapper;
pub mod io;
pub mod imports_exports;
pub mod utility_class;
pub mod module_relationships;

pub trait Detector {
    fn detect(&self, file: &FileCst) -> Vec<Finding>;
}

pub struct DetectorRegistry {
    detectors: Vec<Box<dyn Detector + Send + Sync>>,
}

impl DetectorRegistry {
    pub fn new(cfg: &NarratorConfig) -> Self {
        let detectors: Vec<Box<dyn Detector + Send + Sync>> = vec![
            Box::new(comments::CommentsDetector::new(cfg)),
            Box::new(entrypoint::EntrypointDetector::default()),
            Box::new(wrapper::WrapperDetector::default()),
            Box::new(io::IoDetector::new(cfg)),
            Box::new(imports_exports::ImportsExportsDetector::default()),
            Box::new(utility_class::UtilityClassDetector::default()),
            Box::new(module_relationships::ModuleRelationshipsDetector::default()),
        ];
        Self { detectors }
    }

    pub fn detect_all(&self, file: &FileCst) -> Vec<Finding> {
        let mut out = Vec::new();
        for d in &self.detectors {
            out.extend(d.detect(file));
        }
        out
    }
}