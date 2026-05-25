use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::db::Database;

/// Update source files with new statuses from database
pub fn update_source_files(dir: &Path, db: &Database, marker_prefix: &str) -> Result<()> {
    for item in &db.items {
        let file_path = dir.join(&item.file);
        if !file_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read: {:?}", file_path))?;

        let mut lines: Vec<String> = content.lines().map(String::from).collect();

        if item.line <= lines.len() {
            let line_idx = item.line - 1;
            let line = &lines[line_idx];

            // Find and replace the status in the marker
            // Pattern: <prefix>type:id:old_status -> <prefix>type:id:new_status
            let pattern = format!("{}{}:{}:", marker_prefix, item.item_type, item.id);

            if let Some(pos) = line.find(&pattern) {
                // Find the end of the status (space or end of marker)
                let after_pattern = &line[pos + pattern.len()..];
                let status_end = after_pattern
                    .find([' ', '\t', '\n'])
                    .unwrap_or(after_pattern.len());
                let old_status = &after_pattern[..status_end];

                if old_status != item.status {
                    // Replace the old status with new status
                    let search_str = format!(
                        "{}{}:{}:{}",
                        marker_prefix, item.item_type, item.id, old_status
                    );
                    let replace_str = format!(
                        "{}{}:{}:{}",
                        marker_prefix, item.item_type, item.id, item.status
                    );
                    let new_line = line.replacen(&search_str, &replace_str, 1);
                    lines[line_idx] = new_line;

                    // Write updated content
                    let mut new_content = lines.join("\n");
                    if content.ends_with('\n') {
                        new_content.push('\n');
                    }
                    fs::write(&file_path, &new_content)
                        .with_context(|| format!("Failed to write: {:?}", file_path))?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DbItem;
    use tempfile::TempDir;

    #[test]
    fn test_update_source_files_changes_status() {
        let temp_dir = TempDir::new().unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let file_path = src_dir.join("main.rs");
        fs::write(&file_path, "// $$todo:1:open fix this\n").unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 1,
            status: "done".to_string(),
            content: "fix this".to_string(),
        });

        update_source_files(temp_dir.path(), &db, "$$").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "// $$todo:1:done fix this\n");
        assert!(!content.contains("open"));
    }

    #[test]
    fn test_update_source_files_preserves_unchanged() {
        let temp_dir = TempDir::new().unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let file_path = src_dir.join("main.rs");
        fs::write(&file_path, "// $$todo:1:done fix this\n").unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 1,
            status: "done".to_string(),
            content: "fix this".to_string(),
        });

        update_source_files(temp_dir.path(), &db, "$$").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("$$todo:1:done"));
    }

    #[test]
    fn test_update_source_files_uses_marker_prefix() {
        let temp_dir = TempDir::new().unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let file_path = src_dir.join("main.rs");
        fs::write(&file_path, "// @@todo:1:open fix this\n").unwrap();

        let mut db = Database::default();
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 1,
            status: "done".to_string(),
            content: "fix this".to_string(),
        });

        update_source_files(temp_dir.path(), &db, "@@").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "// @@todo:1:done fix this\n");
    }
}
