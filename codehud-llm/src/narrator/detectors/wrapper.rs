use crate::narrator::{FileCst, Finding, FindingType, Node};
use crate::narrator::detectors::Detector;

/// Detect functions that return a single call: return foo(...)
#[derive(Default)]
pub struct WrapperDetector;

impl Detector for WrapperDetector {
    fn detect(&self, file: &FileCst) -> Vec<Finding> {
        let mut nodes = Vec::new();
        file.root.walk(&mut nodes);
        let mut out = Vec::new();

        for n in nodes {
            if n.is_kind("function_item") || n.is_kind("function_definition") || n.is_kind("function_declaration") {
                // Look for a child block containing a single return_statement -> call_expression
                if let Some((ret, _call, callee)) = self.find_return_call(n) {
                    let finding = Finding::new(&file.path.to_string_lossy(), ret.line(), FindingType::WrapperFunction)
                        .with_owner(self.extract_function_name(n).unwrap_or_else(|| "function".to_string()))
                        .with_subject(callee)
                        .with_lines(vec![ret.line()]);
                    out.push(finding);
                }
            }
        }
        out
    }
}

impl WrapperDetector {
    fn extract_function_name(&self, func: &Node) -> Option<String> {
        // naive: find first child 'identifier'
        for ch in &func.children {
            if ch.is_kind("identifier") {
                return ch.text.clone();
            }
        }
        None
    }

    fn find_return_call<'a>(&self, func: &'a Node) -> Option<(&'a Node, &'a Node, String)> {
        for ch in &func.children {
            if ch.is_kind("block") || ch.is_kind("suite") || ch.is_kind("statement_block") || ch.is_kind("compound_statement") {
                // descend to find return -> call
                let mut stack = vec![ch];
                while let Some(n) = stack.pop() {
                    if n.is_kind("return_statement") || n.is_kind("return_expression") {
                        // child call
                        for g in &n.children {
                            if g.kind.contains("call") || g.is_kind("call_expression") || g.is_kind("call") {
                                // find callee identifier
                                if let Some(name) = self.find_callee_name(g) {
                                    return Some((n, g, name));
                                }
                            }
                        }
                    }
                    for cc in &n.children {
                        stack.push(cc);
                    }
                }
            }
        }
        None
    }

    fn find_callee_name(&self, call: &Node) -> Option<String> {
        // try to find 'identifier' or 'property_identifier'
        let mut flat = Vec::new();
        call.walk(&mut flat);
        let mut parts = Vec::new();
        for n in flat {
            if n.is_kind("identifier") || n.is_kind("property_identifier") || n.is_kind("field_identifier") {
                if let Some(t) = &n.text {
                    parts.push(t.clone());
                }
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("."))
        }
    }
}