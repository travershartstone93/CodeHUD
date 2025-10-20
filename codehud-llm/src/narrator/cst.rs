use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Minimal JSON shape compatible with `tree-sitter parse --json`.
/// We only rely on `type`, `children`, `text`, and `startPoint`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default, rename = "startPoint")]
    pub start_point: Option<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct FileCst {
    pub path: PathBuf,   // original source path if embedded; else the CST file path
    pub root: Node,
    pub source_text: Option<String>, // optional (not always present in CST JSON)
}

// Helper functions for Node
impl Node {
    pub fn line(&self) -> usize {
        self.start_point.as_ref().map(|p| p.row + 1).unwrap_or(1)
    }

    pub fn is_kind(&self, k: &str) -> bool {
        self.kind == k
    }

    pub fn walk<'a>(&'a self, acc: &mut Vec<&'a Node>) {
        acc.push(self);
        for ch in &self.children {
            ch.walk(acc);
        }
    }

    /// Get all text content from this node and its children
    pub fn collect_text(&self) -> String {
        let mut result = String::new();
        self.collect_text_recursive(&mut result);
        result
    }

    fn collect_text_recursive(&self, acc: &mut String) {
        if let Some(t) = &self.text {
            acc.push_str(t);
            acc.push(' ');
        }
        for ch in &self.children {
            ch.collect_text_recursive(acc);
        }
    }
}