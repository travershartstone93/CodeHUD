use crate::narrator::{FileCst, Finding, FindingType, NarratorConfig};
use crate::narrator::detectors::Detector;
use regex::Regex;

pub struct CommentsDetector {
    todo: Regex,
    fixme: Regex,
    note: Regex,
    header_role: bool,
}

impl CommentsDetector {
    pub fn new(cfg: &NarratorConfig) -> Self {
        Self {
            todo: Regex::new(r"(?i)\bTODO\b[:\-]?\s*(.*)").unwrap(),
            fixme: Regex::new(r"(?i)\bFIXME\b[:\-]?\s*(.*)").unwrap(),
            note: Regex::new(r"(?i)\bNOTE\b[:\-]?\s*(.*)").unwrap(),
            header_role: cfg.roles.enable_header_comment_role,
        }
    }
}

impl Detector for CommentsDetector {
    fn detect(&self, file: &FileCst) -> Vec<Finding> {
        // Walk and find "comment" nodes (tree-sitter common name)
        let mut nodes = Vec::new();
        file.root.walk(&mut nodes);

        let mut out = Vec::new();
        // First comment for "Role"
        let mut header_done = !self.header_role;

        for n in nodes {
            if !n.is_kind("comment") { continue; }
            let text = n.text.clone().unwrap_or_default();
            let line = n.line();

            if !header_done {
                // First non-empty trimmed comment line becomes "Note" we map later to "Role" section in render.
                if text.trim().len() > 2 {
                    let cleaned_text = text.trim()
                        .trim_start_matches(|c| c == '#' || c == '/' || c == '*')
                        .trim()
                        .to_string();
                    let finding = Finding::new(&file.path.to_string_lossy(), line, FindingType::NoteComment)
                        .with_text(cleaned_text);
                    out.push(finding);
                    header_done = true;
                }
            }

            if let Some(cap) = self.todo.captures(&text) {
                let todo_text = cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                let finding = Finding::new(&file.path.to_string_lossy(), line, FindingType::TodoComment)
                    .with_text(todo_text);
                out.push(finding);
            } else if let Some(cap) = self.fixme.captures(&text) {
                let fixme_text = cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                let finding = Finding::new(&file.path.to_string_lossy(), line, FindingType::FixmeComment)
                    .with_text(fixme_text);
                out.push(finding);
            } else if let Some(cap) = self.note.captures(&text) {
                let note_text = cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                let finding = Finding::new(&file.path.to_string_lossy(), line, FindingType::NoteComment)
                    .with_text(note_text);
                out.push(finding);
            }
        }
        out
    }
}