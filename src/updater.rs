use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::db::{Database, DbItem};
use crate::scanner::find_markers;

struct MarkerAssignment {
    line_idx: usize,
    column_offset: usize,
    marker_type: String,
    id: u64,
    status: String,
}

/// Update files with new IDs for markers that don't have them
pub fn update_files_with_ids(
    dir: &Path,
    staged_files: &[String],
    db: &mut Database,
    marker_prefix: &str,
) -> Result<()> {
    for file_path in staged_files {
        let full_path = dir.join(file_path);
        if !full_path.exists() {
            continue;
        }

        let markers = find_markers(&full_path, marker_prefix)?;

        // Get lines
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {:?}", full_path))?;
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let original_lines = lines.clone();

        let mut assignments = Vec::new();
        let mut new_items = Vec::new();
        for marker in &markers {
            if !marker.has_id && marker.line_number <= lines.len() {
                let line_idx = marker.line_number - 1;
                let old_marker = format!("{}{}", marker_prefix, marker.marker_type);
                let original_line = &original_lines[line_idx];
                let content_after = original_line[marker.column_offset + old_marker.len()..].trim();

                db.counter += 1;
                let new_id = db.counter;
                let status = "open".to_string();

                new_items.push(DbItem {
                    id: new_id,
                    item_type: marker.marker_type.clone(),
                    file: file_path.clone(),
                    line: marker.line_number,
                    column: marker.column_offset + 1,
                    status,
                    content: content_after.to_string(),
                });

                assignments.push(MarkerAssignment {
                    line_idx,
                    column_offset: marker.column_offset,
                    marker_type: marker.marker_type.clone(),
                    id: new_id,
                    status: "open".to_string(),
                });
            }
        }

        // Replace from right to left so earlier offsets stay valid.
        for assignment in assignments.iter().rev() {
            let old_marker = format!("{}{}", marker_prefix, assignment.marker_type);
            let new_marker = format!(
                "{}{}:{}:{}",
                marker_prefix, assignment.marker_type, assignment.id, assignment.status
            );
            let marker_range =
                assignment.column_offset..assignment.column_offset + old_marker.len();

            if lines[assignment.line_idx].get(marker_range.clone()) == Some(old_marker.as_str()) {
                lines[assignment.line_idx].replace_range(marker_range, &new_marker);
            }
        }

        // Write updated content
        let mut new_content = lines.join("\n");
        if content.ends_with('\n') {
            new_content.push('\n');
        }
        if new_content != content {
            fs::write(&full_path, &new_content)
                .with_context(|| format!("Failed to write file: {:?}", full_path))?;
        }

        // Add new items to database in source order.
        for item in new_items {
            db.items.push(item);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_update_files_assigns_id() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test file with a marker
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "// $$todo fix this bug\n").unwrap();

        let mut db = Database::default();
        let staged_files = vec!["test.rs".to_string()];

        update_files_with_ids(temp_dir.path(), &staged_files, &mut db, "$$").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("$$todo:1:open"));
        assert_eq!(db.counter, 1);
        assert_eq!(db.items.len(), 1);
    }

    #[test]
    fn test_update_files_assigns_same_type_markers_on_one_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "// $$todo first $$todo second\n").unwrap();

        let mut db = Database::default();
        let staged_files = vec!["test.rs".to_string()];

        update_files_with_ids(temp_dir.path(), &staged_files, &mut db, "$$").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "// $$todo:1:open first $$todo:2:open second\n");
        assert_eq!(db.items.len(), 2);
        assert_eq!(db.items[0].id, 1);
        assert_eq!(db.items[0].content, "first $$todo second");
        assert_eq!(db.items[1].id, 2);
        assert_eq!(db.items[1].content, "second");
    }

    #[test]
    fn test_update_files_preserves_existing_id() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test file with an existing ID
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "// $$todo:5:done fix this bug\n").unwrap();

        let mut db = Database::default();
        let staged_files = vec!["test.rs".to_string()];

        update_files_with_ids(temp_dir.path(), &staged_files, &mut db, "$$").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        // Should NOT change the existing ID
        assert!(content.contains("$$todo:5:done"));
        assert_eq!(db.items.len(), 0);
    }

    #[test]
    fn test_update_files_increments_counter() {
        let temp_dir = TempDir::new().unwrap();

        // First file
        let file_path1 = temp_dir.path().join("test1.rs");
        fs::write(&file_path1, "// $$todo first\n").unwrap();

        // Second file
        let file_path2 = temp_dir.path().join("test2.rs");
        fs::write(&file_path2, "// $$fixme second\n").unwrap();

        let mut db = Database::default();
        let staged_files = vec!["test1.rs".to_string(), "test2.rs".to_string()];

        update_files_with_ids(temp_dir.path(), &staged_files, &mut db, "$$").unwrap();

        assert_eq!(db.counter, 2);
    }
}
