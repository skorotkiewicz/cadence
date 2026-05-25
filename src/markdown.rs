use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

use crate::db::{Database, DbItem};
use crate::schema::{Schemas, fallback_status_for_markdown, load_schemas};
use std::collections::HashMap;

struct ParsedMarkdownItem {
    id: u64,
    item_type: String,
    status: Option<String>,
    checkbox: String,
    marker_status: String,
    content: String,
}

/// Generate markdown files from database items
pub fn generate_markdown_files(dir: &Path, db: &Database, marker_prefix: &str) -> Result<()> {
    let items_dir = dir.join(".cadence").join("items");
    fs::create_dir_all(&items_dir)
        .with_context(|| format!("Failed to create directory: {:?}", items_dir))?;
    let schemas = load_schemas(dir)?;

    // Group items by type
    let mut by_type: HashMap<String, Vec<&DbItem>> = HashMap::new();
    for item in &db.items {
        by_type
            .entry(item.item_type.clone())
            .or_default()
            .push(item);
    }

    // Generate markdown for each type
    for (type_name, items) in by_type {
        let md_path = items_dir.join(format!("{}.md", type_name));
        let mut content = String::new();

        for item in items {
            let checked = schemas.markdown_for_status(&item.item_type, &item.status);
            let marker = format!(
                "{}{}:{}:{}",
                marker_prefix, item.item_type, item.id, item.status
            );
            let location = item_location(item);
            let mut item_lines = item.content.lines();
            let first_line = item_lines.next().unwrap_or("");

            content.push_str(&format!(
                "- {} {} - {} - {}\n",
                checked, marker, location, first_line
            ));
            for line in item_lines {
                content.push_str("  ");
                content.push_str(line);
                content.push('\n');
            }
        }

        fs::write(&md_path, content).with_context(|| format!("Failed to write: {:?}", md_path))?;
    }

    Ok(())
}

/// Parse markdown file and return updated statuses
pub fn parse_markdown_status(dir: &Path, db: &mut Database, marker_prefix: &str) -> Result<()> {
    parse_markdown_status_filtered(dir, db, marker_prefix, None)
}

pub fn parse_markdown_status_for_files(
    dir: &Path,
    db: &mut Database,
    marker_prefix: &str,
    files: &[String],
) -> Result<()> {
    parse_markdown_status_filtered(dir, db, marker_prefix, Some(files))
}

fn parse_markdown_status_filtered(
    dir: &Path,
    db: &mut Database,
    marker_prefix: &str,
    files: Option<&[String]>,
) -> Result<()> {
    let items_dir = dir.join(".cadence").join("items");
    if !items_dir.exists() {
        return Ok(());
    }

    let schemas = load_schemas(dir)?;

    // Read each markdown file
    for entry in fs::read_dir(&items_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let type_name = path.file_stem().unwrap().to_string_lossy().to_string();
            let content =
                fs::read_to_string(&path).with_context(|| format!("Failed to read: {:?}", path))?;

            let mut current_item: Option<ParsedMarkdownItem> = None;
            for line in content.lines() {
                if let Some(item) = parse_markdown_item_line(line, marker_prefix, &schemas)? {
                    if let Some(item) = current_item.take() {
                        apply_markdown_item(db, &type_name, item, files)?;
                    }

                    current_item = Some(item);
                } else if let Some(item) = &mut current_item {
                    append_continuation_line(&mut item.content, line);
                }
            }

            if let Some(item) = current_item.take() {
                apply_markdown_item(db, &type_name, item, files)?;
            }
        }
    }

    Ok(())
}

fn parse_markdown_item_line(
    line: &str,
    marker_prefix: &str,
    schemas: &Schemas,
) -> Result<Option<ParsedMarkdownItem>> {
    let Some(rest) = line.strip_prefix("- [") else {
        return Ok(None);
    };
    let Some((checkbox, rest)) = rest.split_once(']') else {
        return Ok(None);
    };
    let checkbox = format!("[{}]", checkbox);

    let Some(marker_start) = rest.find(marker_prefix) else {
        return Ok(None);
    };
    let marker = &rest[marker_start + marker_prefix.len()..];
    let Some((item_type, marker)) = marker.split_once(':') else {
        return Ok(None);
    };
    let Some((id, marker)) = marker.split_once(':') else {
        return Ok(None);
    };
    let Ok(id) = id.parse::<u64>() else {
        return Ok(None);
    };
    let (marker_status, content) = marker.split_once(" - ").unwrap_or((marker, ""));
    let content = strip_location_prefix(content);
    let status = schemas
        .status_for_markdown(item_type, &checkbox)
        .or_else(|| fallback_status_for_markdown(&checkbox).map(str::to_string));
    Ok(Some(ParsedMarkdownItem {
        id,
        item_type: item_type.to_string(),
        checkbox,
        marker_status: marker_status.to_string(),
        status,
        content: content.to_string(),
    }))
}

fn item_location(item: &DbItem) -> String {
    if item.column > 0 {
        format!("{}:{}:{}", item.file, item.line, item.column)
    } else {
        format!("{}:{}", item.file, item.line)
    }
}

fn strip_location_prefix(content: &str) -> &str {
    let Some((candidate, rest)) = content.split_once(" - ") else {
        return content;
    };

    if is_source_location(candidate) {
        rest
    } else {
        content
    }
}

fn is_source_location(value: &str) -> bool {
    let Some((before_last, last)) = value.rsplit_once(':') else {
        return false;
    };
    if before_last.is_empty() || !last.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    if let Some((path, line)) = before_last.rsplit_once(':') {
        !path.is_empty() && !line.is_empty() && line.chars().all(|c| c.is_ascii_digit())
    } else {
        !before_last.is_empty()
    }
}

fn append_continuation_line(content: &mut String, line: &str) {
    let line = line
        .strip_prefix("  ")
        .or_else(|| line.strip_prefix('\t'))
        .unwrap_or(line);

    if !content.is_empty() {
        content.push('\n');
    }

    content.push_str(line);
}

fn apply_markdown_item(
    db: &mut Database,
    type_name: &str,
    parsed: ParsedMarkdownItem,
    files: Option<&[String]>,
) -> Result<()> {
    if parsed.item_type != type_name {
        return Ok(());
    }

    if let Some(item) = db.items.iter_mut().find(|item| {
        item.id == parsed.id
            && item.item_type == type_name
            && files
                .map(|files| files.contains(&item.file))
                .unwrap_or(true)
    }) {
        let Some(status) = parsed.status else {
            bail!(
                "Unknown checklist marker `{}` for `{}`; add it to .cadence/schemas.yml or keep status `{}`",
                parsed.checkbox,
                parsed.item_type,
                parsed.marker_status
            );
        };

        item.status = status;
        item.content = parsed.content;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_markdown_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            column: 5,
            status: "open".to_string(),
            content: "Fix bug".to_string(),
        });

        generate_markdown_files(temp_dir.path(), &db, "$$").unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        assert!(md_path.exists());

        let content = fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("[ ]"));
        assert!(content.contains("$$todo:1:open"));
        assert!(content.contains("src/main.rs:10:5"));
    }

    #[test]
    fn test_generate_markdown_done_status() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 2,
            item_type: "todo".to_string(),
            file: "src/lib.rs".to_string(),
            line: 20,
            column: 0,
            status: "done".to_string(),
            content: "Done task".to_string(),
        });

        generate_markdown_files(temp_dir.path(), &db, "$$").unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        let content = fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("[x]"));
    }

    #[test]
    fn test_generate_markdown_uses_marker_prefix() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 2,
            item_type: "todo".to_string(),
            file: "src/lib.rs".to_string(),
            line: 20,
            column: 0,
            status: "done".to_string(),
            content: "Done task".to_string(),
        });

        generate_markdown_files(temp_dir.path(), &db, "@@").unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        let content = fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("@@todo:2:done"));
        assert!(!content.contains("$$todo:2:done"));
    }

    #[test]
    fn test_generate_markdown_uses_schema_status_marker() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();
        fs::write(
            temp_dir.path().join(".cadence").join("schemas.yml"),
            r#"todo:
  statuses: ["open:[ ]", "done:[X]", "in-progress:[Q]"]
"#,
        )
        .unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 2,
            item_type: "todo".to_string(),
            file: "src/lib.rs".to_string(),
            line: 20,
            column: 7,
            status: "in-progress".to_string(),
            content: "Done task".to_string(),
        });

        generate_markdown_files(temp_dir.path(), &db, "$$").unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        let content = fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("- [Q] $$todo:2:in-progress - src/lib.rs:20:7 - Done task"));
    }

    #[test]
    fn test_generate_markdown_multiline_content() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 3,
            item_type: "todo".to_string(),
            file: "src/lib.rs".to_string(),
            line: 20,
            column: 0,
            status: "done".to_string(),
            content: "open final flux\nadd\nsupport\nfor\nmulti\nlines".to_string(),
        });

        generate_markdown_files(temp_dir.path(), &db, "$$").unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        let content = fs::read_to_string(&md_path).unwrap();
        assert_eq!(
            content,
            "- [x] $$todo:3:done - src/lib.rs:20 - open final flux\n  add\n  support\n  for\n  multi\n  lines\n"
        );
    }

    #[test]
    fn test_parse_markdown_status_updates_db() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();

        // Create markdown file
        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        fs::write(
            &md_path,
            "- [x] $$todo:1:done - src/main.rs:10:5 - Fix bug\n",
        )
        .unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            column: 0,
            status: "open".to_string(),
            content: "Fix bug".to_string(),
        });

        parse_markdown_status(temp_dir.path(), &mut db, "$$").unwrap();

        assert_eq!(db.items[0].status, "done");
        assert_eq!(db.items[0].content, "Fix bug");
    }

    #[test]
    fn test_parse_markdown_status_uses_marker_prefix() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        fs::write(&md_path, "- [x] @@todo:1:done - Fix bug\n").unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            column: 0,
            status: "open".to_string(),
            content: "Fix bug".to_string(),
        });

        parse_markdown_status(temp_dir.path(), &mut db, "@@").unwrap();

        assert_eq!(db.items[0].status, "done");
    }

    #[test]
    fn test_parse_markdown_status_uses_schema_status_marker() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();
        fs::write(
            temp_dir.path().join(".cadence").join("schemas.yml"),
            r#"todo:
  statuses: ["open:[ ]", "done:[X]", "in-progress:[Q]"]
"#,
        )
        .unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        fs::write(&md_path, "- [Q] $$todo:1:open - Fix bug\n").unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            column: 0,
            status: "open".to_string(),
            content: "Fix bug".to_string(),
        });

        parse_markdown_status(temp_dir.path(), &mut db, "$$").unwrap();

        assert_eq!(db.items[0].status, "in-progress");
    }

    #[test]
    fn test_parse_markdown_multiline_content_updates_db() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence").join("items")).unwrap();

        let md_path = temp_dir
            .path()
            .join(".cadence")
            .join("items")
            .join("todo.md");
        fs::write(
            &md_path,
            concat!(
                "- [ ] $$todo:1:open - test\n",
                "- [x] $$todo:3:done - open final flux\n",
                "  add\n",
                "  support\n",
                "  for\n",
                "  multi\n",
                "  lines\n",
                "- [x] $$todo:5:done - antimater + mater\n",
                "Hello\n",
                "World\n",
            ),
        )
        .unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            column: 0,
            status: "open".to_string(),
            content: "test".to_string(),
        });
        db.items.push(DbItem {
            id: 3,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 12,
            column: 0,
            status: "open".to_string(),
            content: "open final flux".to_string(),
        });
        db.items.push(DbItem {
            id: 5,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 14,
            column: 0,
            status: "open".to_string(),
            content: "antimater + mater".to_string(),
        });

        parse_markdown_status(temp_dir.path(), &mut db, "$$").unwrap();

        assert_eq!(db.items[0].content, "test");
        assert_eq!(db.items[1].status, "done");
        assert_eq!(
            db.items[1].content,
            "open final flux\nadd\nsupport\nfor\nmulti\nlines"
        );
        assert_eq!(db.items[2].status, "done");
        assert_eq!(db.items[2].content, "antimater + mater\nHello\nWorld");
    }
}
