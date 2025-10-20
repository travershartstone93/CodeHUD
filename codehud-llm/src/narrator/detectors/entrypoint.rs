use crate::narrator::{FileCst, Finding, FindingType, Node};
use crate::narrator::detectors::Detector;

#[derive(Default)]
pub struct EntrypointDetector;

impl Detector for EntrypointDetector {
    fn detect(&self, file: &FileCst) -> Vec<Finding> {
        // Look for Python __name__ == "__main__" pattern and Rust main function
        let mut nodes = Vec::new();
        file.root.walk(&mut nodes);

        for n in nodes {
            // Python entrypoint pattern
            if n.is_kind("if_statement") {
                let text_bag = n.collect_text();
                if text_bag.contains("__name__") && text_bag.contains("__main__") {
                    let finding = Finding::new(&file.path.to_string_lossy(), n.line(), FindingType::EntrypointScript);
                    return vec![finding];
                }
            }

            // Rust main function
            if n.is_kind("function_item") || n.is_kind("function_definition") {
                if let Some(name) = self.extract_function_name(n) {
                    if name == "main" {
                        let finding = Finding::new(&file.path.to_string_lossy(), n.line(), FindingType::EntrypointScript);
                        return vec![finding];
                    }
                }
            }
        }
        vec![]
    }
}

impl EntrypointDetector {
    fn extract_function_name(&self, func_node: &Node) -> Option<String> {
        for ch in &func_node.children {
            if ch.is_kind("identifier") {
                return ch.text.clone();
            }
        }
        None
    }
}