use crate::narrator::{FileDoc, NarratorConfig};

pub fn render_markdown(_rel_path: &str, doc: &FileDoc, _cfg: &NarratorConfig) -> String {
    let mut out = String::new();

    // Don't include the file path header, we'll integrate this differently
    if let Some(role) = &doc.role_line {
        out.push_str(&format!("Role: {}\n", ensure_period(role)));
    }

    for section in doc.sections.values() {
        out.push_str(&format!("{}\n", section.title));
        for b in &section.bullets {
            let line = if b.starts_with("- ") {
                b.clone()
            } else {
                format!("- {}", ensure_period(b))
            };
            out.push_str(&format!("{}\n", line));
        }
        out.push('\n');
    }
    out
}

/// Render bullet points in a compact format for LLM consumption
pub fn render_bullets_compact(doc: &FileDoc) -> String {
    let mut out = String::new();

    if let Some(role) = &doc.role_line {
        out.push_str(&format!("Role: {}\n", ensure_period(role)));
    }

    for section in doc.sections.values() {
        if section.bullets.is_empty() {
            continue;
        }

        out.push_str(&format!("{}:\n", section.title));
        for bullet in &section.bullets {
            let clean_bullet = bullet
                .trim_start_matches("- ")
                .trim();
            out.push_str(&format!("- {}\n", ensure_period(clean_bullet)));
        }
        out.push('\n');
    }

    out.trim().to_string()
}

fn ensure_period(s: &str) -> String {
    let t = s.trim();
    if t.ends_with('.') {
        t.to_string()
    } else {
        format!("{t}.")
    }
}