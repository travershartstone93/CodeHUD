use crate::narrator::{FileCst, Finding, FindingType};
use crate::narrator::detectors::Detector;

#[derive(Default)]
pub struct ImportsExportsDetector;

impl Detector for ImportsExportsDetector {
    fn detect(&self, file: &FileCst) -> Vec<Finding> {
        let mut nodes = Vec::new();
        file.root.walk(&mut nodes);
        let mut imports = vec![];
        let mut exports = vec![];

        for n in nodes {
            match n.kind.as_str() {
                // Python/Java
                "import_statement" | "import_from_statement" => {
                    if let Some(txt) = &n.text {
                        imports.push(txt.trim().to_string());
                    }
                }
                // JavaScript/TypeScript/Go
                "import_declaration" => {
                    if let Some(txt) = &n.text {
                        imports.push(txt.trim().to_string());
                    }
                }
                // Go specific
                "import_spec" => {
                    if let Some(txt) = &n.text {
                        imports.push(txt.trim().to_string());
                    }
                }
                "export_statement" | "export_clause" | "export_declaration" => {
                    if let Some(txt) = &n.text {
                        exports.push(txt.trim().to_string());
                    }
                }
                // Rust
                "use_declaration" | "extern_crate_declaration" => {
                    if let Some(txt) = &n.text {
                        imports.push(txt.trim().to_string());
                    }
                }
                _ => {}
            }
        }

        let mut out = vec![];
        if !exports.is_empty() {
            // Create separate findings for each export to preserve granularity
            for export in exports {
                let finding = Finding::new(&file.path.to_string_lossy(), 1, FindingType::ExportSymbol)
                    .with_subject(export);
                out.push(finding);
            }
        }
        if !imports.is_empty() {
            // Create separate findings for each import to preserve granularity and show relationships
            for import in imports {
                let finding = Finding::new(&file.path.to_string_lossy(), 1, FindingType::ImportSymbol)
                    .with_subject(import);
                out.push(finding);
            }
        }
        out
    }
}