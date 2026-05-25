use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::db::{Database, DbItem};
use std::collections::HashMap;

/// Generate markdown files from database items
pub fn generate_markdown_files(dir: &Path, db: &Database) -> Result<()> {
    let cadence_dir = dir.join(".cadence");

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
        let md_path = cadence_dir.join(format!("{}.md", type_name));
        let mut content = String::new();

        for item in items {
            let checked = if item.status == "done" { "[x]" } else { "[ ]" };
            let marker = format!("$${}:{}:{}", item.item_type, item.id, item.status);
            content.push_str(&format!("- {} {} - {}\n", checked, marker, item.content));
        }

        fs::write(&md_path, content).with_context(|| format!("Failed to write: {:?}", md_path))?;
    }

    Ok(())
}

/// Parse markdown file and return updated statuses
pub fn parse_markdown_status(dir: &Path, db: &mut Database) -> Result<()> {
    let cadence_dir = dir.join(".cadence");

    // Read each markdown file
    for entry in fs::read_dir(&cadence_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let type_name = path.file_stem().unwrap().to_string_lossy().to_string();
            let content =
                fs::read_to_string(&path).with_context(|| format!("Failed to read: {:?}", path))?;

            // Parse each line
            for line in content.lines() {
                // Parse: - [x] $$todo:1:done - content
                if line.starts_with("- [") {
                    let checked = line.starts_with("- [x]");
                    let status = if checked { "done" } else { "open" };

                    // Extract marker from line
                    if let Some(marker_start) = line.find("$$") {
                        let rest = &line[marker_start..];
                        // Parse $$type:id:status
                        let parts: Vec<&str> = rest.split(':').collect();
                        if parts.len() >= 3
                            && let Ok(id) = parts[1].parse::<u64>()
                        {
                            // Update database
                            for item in &mut db.items {
                                if item.id == id && item.item_type == type_name {
                                    item.status = status.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
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
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            status: "open".to_string(),
            content: "Fix bug".to_string(),
        });

        generate_markdown_files(temp_dir.path(), &db).unwrap();

        let md_path = temp_dir.path().join(".cadence").join("todo.md");
        assert!(md_path.exists());

        let content = fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("[ ]"));
        assert!(content.contains("$$todo:1:open"));
    }

    #[test]
    fn test_generate_markdown_done_status() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 2,
            item_type: "todo".to_string(),
            file: "src/lib.rs".to_string(),
            line: 20,
            status: "done".to_string(),
            content: "Done task".to_string(),
        });

        generate_markdown_files(temp_dir.path(), &db).unwrap();

        let md_path = temp_dir.path().join(".cadence").join("todo.md");
        let content = fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("[x]"));
    }

    #[test]
    fn test_parse_markdown_status_updates_db() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();

        // Create markdown file
        let md_path = temp_dir.path().join(".cadence").join("todo.md");
        fs::write(&md_path, "- [x] $$todo:1:done - Fix bug\n").unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            status: "open".to_string(),
            content: "Fix bug".to_string(),
        });

        parse_markdown_status(temp_dir.path(), &mut db).unwrap();

        assert_eq!(db.items[0].status, "done");
    }
}
