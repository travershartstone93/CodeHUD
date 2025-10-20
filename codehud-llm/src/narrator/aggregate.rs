use crate::narrator::{Finding, FindingType, NarratorConfig};
use itertools::Itertools;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct SectionDoc {
    pub title: String,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileDoc {
    pub file: String,
    pub sections: BTreeMap<String, SectionDoc>,
    pub role_line: Option<String>,
}

pub fn aggregate_findings(file: &str, findings: &[Finding], cfg: &NarratorConfig) -> FileDoc {
    let mut doc = FileDoc {
        file: file.to_string(),
        sections: BTreeMap::new(),
        role_line: None,
    };

    // Role line: first NoteComment becomes "Role" sentence (trimmed)
    if let Some(f) = findings
        .iter()
        .find(|f| f.typ == FindingType::NoteComment && f.text.as_deref().unwrap_or("").len() > 0)
    {
        doc.role_line = Some(capitalize_sentence(f.text.as_ref().unwrap()));
    }

    // Buckets
    let mut risks = vec![];
    let mut entry = vec![];
    let mut structure = vec![];
    let mut io_net = vec![];
    let mut io_db = vec![];
    let mut io_fs = vec![];
    let mut imports = vec![];
    let mut exports = vec![];
    let mut tests = vec![];

    // Group same (type, owner, subject) and merge lines
    let grouped = findings.iter().into_group_map_by(|f| {
        (
            std::mem::discriminant(&f.typ),
            f.owner.clone(),
            f.subject.clone(),
        )
    });

    for ((_disc, owner, subject), items) in grouped {
        let mut lines: Vec<usize> = items.iter().flat_map(|i| i.lines.clone()).collect();
        lines.sort_unstable();
        lines.dedup();
        let range = fmt_ranges(group_ranges(&lines));
        let first = items[0].clone();

        match first.typ {
            FindingType::EntrypointScript => {
                entry.push(format_template(&cfg.templates.entrypoint_script, &[("range", range.as_str())]))
            }
            FindingType::WrapperFunction => structure.push(
                cfg.templates
                    .wrapper_function
                    .replace("{owner}", owner.as_deref().unwrap_or("this function"))
                    .replace("{subject}", subject.as_deref().unwrap_or("callee"))
                    .replace("{range}", &range),
            ),
            FindingType::NetworkCall => io_net.push(
                cfg.templates
                    .network_call
                    .replace("{owner}", owner.as_deref().unwrap_or("this scope"))
                    .replace("{subject}", subject.as_deref().unwrap_or("network call"))
                    .replace("{range}", &range),
            ),
            FindingType::DbCall => io_db.push(
                cfg.templates
                    .db_call
                    .replace("{owner}", owner.as_deref().unwrap_or("this scope"))
                    .replace("{subject}", subject.as_deref().unwrap_or("db call"))
                    .replace("{range}", &range),
            ),
            FindingType::FsIo => io_fs.push(
                cfg.templates
                    .fs_io
                    .replace("{owner}", owner.as_deref().unwrap_or("this scope"))
                    .replace("{subject}", subject.as_deref().unwrap_or("fs io"))
                    .replace("{range}", &range),
            ),
            FindingType::ExportSymbol => exports.push(render_list(
                &cfg.templates.export_symbol,
                subject.as_deref().unwrap_or(""),
                cfg.limits.exports_max,
            )),
            FindingType::ImportSymbol => imports.push(render_list(
                &cfg.templates.import_symbol,
                subject.as_deref().unwrap_or(""),
                cfg.limits.imports_max,
            )),
            FindingType::IntraCrateImport => imports.push(
                subject.as_deref().unwrap_or("").to_string()
            ),
            FindingType::FunctionCall => structure.push(format!(
                "calls {}",
                subject.as_deref().unwrap_or("function")
            )),
            FindingType::ReExport => exports.push(
                subject.as_deref().unwrap_or("").to_string()
            ),
            FindingType::TodoComment => risks.push(
                cfg.templates
                    .todo_comment
                    .replace("{line}", &first.line.to_string())
                    .replace("{text}", first.text.as_deref().unwrap_or("").trim()),
            ),
            FindingType::FixmeComment => risks.push(
                cfg.templates
                    .fixme_comment
                    .replace("{line}", &first.line.to_string())
                    .replace("{text}", first.text.as_deref().unwrap_or("").trim()),
            ),
            FindingType::NoteComment => {
                // handled as role
            }
            FindingType::StaticUtilityClass => structure.push(
                cfg.templates
                    .static_utility_class
                    .replace("{subject}", subject.as_deref().unwrap_or("class")),
            ),
            FindingType::TestCase => tests.push(format!("- Test case at line {}", first.line)),
        }
    }

    // Install sections in configured order
    for key in &cfg.sections.order {
        match key.as_str() {
            "role" => {
                // rendered in render.rs header
            }
            "entrypoint" => add_if_nonempty(&mut doc, "Entrypoint", std::mem::take(&mut entry)),
            "exports" => add_if_nonempty(&mut doc, "Exports", std::mem::take(&mut exports)),
            "risks" => add_if_nonempty(&mut doc, "Risks", std::mem::take(&mut risks)),
            "io_network" => add_if_nonempty(&mut doc, "Network I/O", std::mem::take(&mut io_net)),
            "io_db" => add_if_nonempty(&mut doc, "Database I/O", std::mem::take(&mut io_db)),
            "io_fs" => add_if_nonempty(&mut doc, "File I/O", std::mem::take(&mut io_fs)),
            "structure" => add_if_nonempty(&mut doc, "Structure", std::mem::take(&mut structure)),
            "imports" => add_if_nonempty(&mut doc, "Imports", std::mem::take(&mut imports)),
            "tests" => add_if_nonempty(&mut doc, "Tests", std::mem::take(&mut tests)),
            _ => {}
        }
    }

    doc
}

fn add_if_nonempty(doc: &mut FileDoc, title: &str, bullets: Vec<String>) {
    if bullets.is_empty() {
        return;
    }
    doc.sections.insert(
        title.to_string(),
        SectionDoc {
            title: title.to_string(),
            bullets,
        },
    );
}

fn capitalize_sentence(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => s.to_string(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn group_ranges(lines: &[usize]) -> Vec<(usize, usize)> {
    if lines.is_empty() {
        return vec![];
    }
    let mut out = vec![];
    let mut start = lines[0];
    let mut last = lines[0];
    for &l in &lines[1..] {
        if l == last + 1 {
            last = l;
        } else {
            out.push((start, last));
            start = l;
            last = l;
        }
    }
    out.push((start, last));
    out
}

fn fmt_ranges(ranges: Vec<(usize, usize)>) -> String {
    ranges
        .into_iter()
        .map(|(a, b)| if a == b { a.to_string() } else { format!("{a}–{b}") })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_list(tpl: &str, csv: &str, max_items: usize) -> String {
    let items: Vec<&str> = csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let (head, tail) = if items.len() > max_items {
        (
            items[..max_items].join(", "),
            Some(items.len() - max_items),
        )
    } else {
        (items.join(", "), None)
    };
    let list = if let Some(n) = tail {
        format!("{head} (+{n} more)")
    } else {
        head
    };
    tpl.replace("{list}", &list)
}

fn format_template(tpl: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = tpl.to_string();
    for (k, v) in pairs {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}