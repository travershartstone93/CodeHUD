use crate::narrator::{FileCst, Finding, FindingType};
use crate::narrator::detectors::Detector;

#[derive(Default)]
pub struct ModuleRelationshipsDetector;

impl Detector for ModuleRelationshipsDetector {
    fn detect(&self, file: &FileCst) -> Vec<Finding> {
        let mut nodes = Vec::new();
        file.root.walk(&mut nodes);
        let mut intra_crate_imports = vec![];
        let mut re_exports = vec![];
        let mut function_calls = vec![];

        for n in nodes {
            match n.kind.as_str() {
                // Rust use declarations
                "use_declaration" => {
                    if let Some(txt) = &n.text {
                        let trimmed = txt.trim();

                        // Detect intra-crate imports
                        if trimmed.contains("use crate::") || trimmed.contains("use super::") || trimmed.contains("use self::") {
                            intra_crate_imports.push(trimmed.to_string());
                        }

                        // Detect re-exports (pub use)
                        if trimmed.starts_with("pub use") {
                            re_exports.push(trimmed.to_string());
                        }
                    }
                }

                // Function/method calls
                "call_expression" => {
                    if let Some(txt) = &n.text {
                        let trimmed = txt.trim();
                        // Look for calls that reference modules (contains ::)
                        if trimmed.contains("::") && !trimmed.starts_with("//") {
                            // Extract just the function call, limit length to avoid huge expressions
                            if trimmed.len() < 200 {
                                // Try to extract the module::function part
                                if let Some(call_part) = extract_module_call(trimmed) {
                                    function_calls.push(call_part);
                                }
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        let mut out = vec![];

        // Add intra-crate imports
        for import in intra_crate_imports {
            let finding = Finding::new(&file.path.to_string_lossy(), 1, FindingType::IntraCrateImport)
                .with_subject(import);
            out.push(finding);
        }

        // Add re-exports
        for re_export in re_exports {
            let finding = Finding::new(&file.path.to_string_lossy(), 1, FindingType::ReExport)
                .with_subject(re_export);
            out.push(finding);
        }

        // Add function calls (deduplicate)
        let mut seen_calls = std::collections::HashSet::new();
        for call in function_calls {
            if seen_calls.insert(call.clone()) {
                let finding = Finding::new(&file.path.to_string_lossy(), 1, FindingType::FunctionCall)
                    .with_subject(call);
                out.push(finding);
            }
        }

        out
    }
}

/// Extract the module::function part from a call expression
fn extract_module_call(call_expr: &str) -> Option<String> {
    // Look for patterns like: module::function(), crate::module::Type::new(), self::helper()
    let parts: Vec<&str> = call_expr.split('(').collect();
    if parts.is_empty() {
        return None;
    }

    let call_path = parts[0].trim();

    // Only include calls that reference modules (contain ::) and look like project code
    if call_path.contains("::") {
        // Filter out standard library and common external crates to reduce noise
        if call_path.starts_with("std::")
            || call_path.starts_with("Vec::")
            || call_path.starts_with("String::")
            || call_path.starts_with("Option::")
            || call_path.starts_with("Result::") {
            return None;
        }

        Some(call_path.to_string())
    } else {
        None
    }
}