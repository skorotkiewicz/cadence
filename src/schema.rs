use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSchema {
    pub name: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkerSchema {
    pub statuses: Vec<StatusSchema>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schemas {
    by_type: HashMap<String, MarkerSchema>,
}

impl Schemas {
    pub fn markdown_for_status(&self, item_type: &str, status: &str) -> String {
        self.by_type
            .get(item_type)
            .and_then(|schema| {
                schema
                    .statuses
                    .iter()
                    .find(|candidate| candidate.name == status)
            })
            .map(|status| status.markdown.clone())
            .unwrap_or_else(|| default_markdown_for_status(status).to_string())
    }

    pub fn status_for_markdown(&self, item_type: &str, markdown: &str) -> Option<String> {
        self.by_type.get(item_type).and_then(|schema| {
            schema
                .statuses
                .iter()
                .find(|candidate| candidate.markdown == markdown)
                .map(|status| status.name.clone())
        })
    }
}

pub fn load_schemas(dir: &Path) -> Result<Schemas> {
    let schemas_path = dir.join(".cadence").join("schemas.yml");
    if !schemas_path.exists() {
        return Ok(Schemas::default());
    }

    let content = fs::read_to_string(&schemas_path)
        .with_context(|| format!("Failed to read: {:?}", schemas_path))?;

    Ok(parse_schemas(&content))
}

fn parse_schemas(content: &str) -> Schemas {
    let mut schemas = Schemas::default();
    let mut current_type = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !line.starts_with(' ') && trimmed.ends_with(':') {
            let type_name = trimmed.trim_end_matches(':').to_string();
            schemas.by_type.entry(type_name.clone()).or_default();
            current_type = Some(type_name);
            continue;
        }

        let Some(type_name) = &current_type else {
            continue;
        };

        if let Some(value) = trimmed.strip_prefix("statuses:") {
            let statuses = parse_statuses(value.trim());
            schemas
                .by_type
                .insert(type_name.clone(), MarkerSchema { statuses });
        }
    }

    schemas
}

fn parse_statuses(value: &str) -> Vec<StatusSchema> {
    let value = value.trim();
    let Some(value) = value.strip_prefix('[') else {
        return Vec::new();
    };
    let Some(value) = value.strip_suffix(']') else {
        return Vec::new();
    };

    split_inline_list(value)
        .into_iter()
        .filter_map(|entry| parse_status_entry(&entry))
        .collect()
}

fn split_inline_list(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for c in value.chars() {
        match (c, quote) {
            ('"' | '\'', None) => quote = Some(c),
            (c, Some(active_quote)) if c == active_quote => quote = None,
            (',', None) => {
                items.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        items.push(current.trim().to_string());
    }

    items
}

fn parse_status_entry(entry: &str) -> Option<StatusSchema> {
    let entry = unquote(entry.trim());
    if entry.is_empty() {
        return None;
    }

    let (name, markdown) = if let Some((name, markdown)) = entry.split_once(':') {
        (name.trim(), markdown.trim())
    } else if let Some(markdown_start) = entry.rfind('[') {
        let (name, markdown) = entry.split_at(markdown_start);
        (name.trim(), markdown.trim())
    } else {
        (entry, default_markdown_for_status(entry))
    };

    if name.is_empty() || markdown.is_empty() {
        return None;
    }

    Some(StatusSchema {
        name: name.to_string(),
        markdown: markdown.to_string(),
    })
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

pub fn default_markdown_for_status(status: &str) -> &'static str {
    if status.eq_ignore_ascii_case("done") {
        "[x]"
    } else {
        "[ ]"
    }
}

pub fn fallback_status_for_markdown(markdown: &str) -> Option<&'static str> {
    match markdown {
        "[ ]" => Some("open"),
        "[x]" | "[X]" => Some("done"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schemas_reads_markdown_markers() {
        let schemas = parse_schemas(
            r#"
todo:
  statuses: ["open:[ ]", "done:[X]", "in-progress:[Q]"]
fixme:
  statuses: ["open:[ ]", "done:[X]"]
"#,
        );

        assert_eq!(schemas.markdown_for_status("todo", "open"), "[ ]");
        assert_eq!(schemas.markdown_for_status("todo", "done"), "[X]");
        assert_eq!(schemas.markdown_for_status("todo", "in-progress"), "[Q]");
        assert_eq!(
            schemas.status_for_markdown("todo", "[Q]"),
            Some("in-progress".to_string())
        );
    }

    #[test]
    fn test_parse_schemas_supports_status_marker_without_colon() {
        let schemas = parse_schemas(
            r#"
todo:
  statuses: ["open:[ ]", "done:[X]", "in-progress[Q]"]
"#,
        );

        assert_eq!(schemas.markdown_for_status("todo", "in-progress"), "[Q]");
    }

    #[test]
    fn test_parse_schemas_keeps_old_status_only_format() {
        let schemas = parse_schemas(
            r#"
todo:
  statuses: ["open", "done", "in-progress"]
"#,
        );

        assert_eq!(schemas.markdown_for_status("todo", "open"), "[ ]");
        assert_eq!(schemas.markdown_for_status("todo", "done"), "[x]");
        assert_eq!(schemas.markdown_for_status("todo", "in-progress"), "[ ]");
    }
}
