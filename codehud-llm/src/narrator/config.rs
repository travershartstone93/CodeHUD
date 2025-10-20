use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarratorConfig {
    pub sections: Sections,
    pub templates: Templates,
    pub limits: Limits,
    pub io: IoLists,
    pub roles: RoleRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sections {
    pub order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Templates {
    pub wrapper_function: String,
    pub network_call: String,
    pub db_call: String,
    pub fs_io: String,
    pub entrypoint_script: String,
    pub todo_comment: String,
    pub fixme_comment: String,
    pub note_comment: String,
    pub export_symbol: String,
    pub import_symbol: String,
    pub static_utility_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    pub todos_per_file: usize,
    pub imports_max: usize,
    pub exports_max: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoLists {
    pub network_callees: Vec<String>,
    pub db_callees: Vec<String>,
    pub fs_callees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRules {
    pub enable_header_comment_role: bool,
}

impl Default for NarratorConfig {
    fn default() -> Self {
        Self {
            sections: Sections {
                order: vec![
                    "role".to_string(),
                    "entrypoint".to_string(),
                    "exports".to_string(),
                    "risks".to_string(),
                    "io_network".to_string(),
                    "io_db".to_string(),
                    "io_fs".to_string(),
                    "structure".to_string(),
                    "imports".to_string(),
                    "tests".to_string(),
                ],
            },
            templates: Templates {
                wrapper_function: "{owner} is a wrapper for {subject} (lines {range}).".to_string(),
                network_call: "{owner} calls {subject} (lines {range}).".to_string(),
                db_call: "{owner} touches DB via {subject} (lines {range}).".to_string(),
                fs_io: "{owner} performs file I/O via {subject} (lines {range}).".to_string(),
                entrypoint_script: "Entrypoint script (lines {range}).".to_string(),
                todo_comment: "TODO (line {line}): {text}".to_string(),
                fixme_comment: "FIXME (line {line}): {text}".to_string(),
                note_comment: "NOTE (line {line}): {text}".to_string(),
                export_symbol: "Exports: {list}".to_string(),
                import_symbol: "Imports: {list}".to_string(),
                static_utility_class: "Utility class: {subject}.".to_string(),
            },
            limits: Limits {
                todos_per_file: 5,
                imports_max: 10,
                exports_max: 10,
            },
            io: IoLists {
                network_callees: vec![
                    "fetch".to_string(),
                    "axios".to_string(),
                    "requests.get".to_string(),
                    "requests.post".to_string(),
                    "http.request".to_string(),
                    "urllib.request".to_string(),
                ],
                db_callees: vec![
                    "cursor.execute".to_string(),
                    "Session.query".to_string(),
                    "prisma".to_string(),
                    "sequelize".to_string(),
                    "typeorm".to_string(),
                ],
                fs_callees: vec![
                    "open".to_string(),
                    "read".to_string(),
                    "write".to_string(),
                    "fs.readFile".to_string(),
                    "fs.writeFile".to_string(),
                    "fs.open".to_string(),
                ],
            },
            roles: RoleRules {
                enable_header_comment_role: true,
            },
        }
    }
}