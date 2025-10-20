use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FindingType {
    TodoComment,
    FixmeComment,
    NoteComment,
    EntrypointScript,
    WrapperFunction,
    NetworkCall,
    DbCall,
    FsIo,
    ExportSymbol,
    ImportSymbol,
    StaticUtilityClass,
    TestCase,
    IntraCrateImport,  // use crate::, use super::, use self::
    FunctionCall,       // Calls to functions from other modules
    ReExport,          // pub use statements
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub typ: FindingType,
    pub subject: Option<String>,
    pub owner: Option<String>,
    pub lines: Vec<usize>,
    pub notes: Vec<String>,
    pub text: Option<String>, // e.g., TODO text
    pub extra: serde_json::Value,
}

impl Finding {
    pub fn new(file: &str, line: usize, typ: FindingType) -> Self {
        Self {
            file: file.to_string(),
            line,
            typ,
            subject: None,
            owner: None,
            lines: vec![line],
            notes: vec![],
            text: None,
            extra: serde_json::json!({}),
        }
    }

    pub fn with_subject(mut self, subject: String) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn with_owner(mut self, owner: String) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = Some(text);
        self
    }

    pub fn with_lines(mut self, lines: Vec<usize>) -> Self {
        self.lines = lines;
        self
    }
}