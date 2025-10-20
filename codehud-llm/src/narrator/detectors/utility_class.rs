use crate::narrator::{FileCst, Finding, FindingType, Node};
use crate::narrator::detectors::Detector;

#[derive(Default)]
pub struct UtilityClassDetector;

impl Detector for UtilityClassDetector {
    fn detect(&self, file: &FileCst) -> Vec<Finding> {
        let mut nodes = Vec::new();
        file.root.walk(&mut nodes);
        let mut out = Vec::new();

        let class_nodes: Vec<&Node> = nodes
            .into_iter()
            .filter(|n| n.is_kind("class_definition") || n.is_kind("class_declaration") || n.is_kind("struct_item"))
            .collect();

        for class in class_nodes {
            let mut methods = 0usize;
            let mut static_like = 0usize;

            for ch in &class.children {
                if ch.kind.contains("method")
                    || ch.is_kind("function_definition")
                    || ch.is_kind("function_item")
                    || ch.is_kind("associated_function") {
                    methods += 1;
                    // crude: look for 'static' token or @staticmethod nearby
                    let text_bag = ch.collect_text();
                    if text_bag.contains("@staticmethod")
                        || text_bag.contains("static ")
                        || text_bag.contains("pub fn ")
                        || text_bag.contains("fn ") {
                        static_like += 1;
                    }
                }
            }

            if methods > 0 && methods == static_like {
                let finding = Finding::new(&file.path.to_string_lossy(), class.line(), FindingType::StaticUtilityClass)
                    .with_subject(self.extract_class_name(class).unwrap_or_else(|| "class".to_string()));
                out.push(finding);
            }
        }
        out
    }
}

impl UtilityClassDetector {
    fn extract_class_name(&self, class: &Node) -> Option<String> {
        for ch in &class.children {
            if ch.is_kind("identifier") || ch.is_kind("type_identifier") {
                return ch.text.clone();
            }
        }
        None
    }
}