use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::db::{Database, DbItem};
use crate::scanner::find_markers;

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
        
        // Process markers that don't have IDs (in reverse order to maintain positions)
        let mut new_items = Vec::new();
        for marker in markers.iter().rev() {
            if !marker.has_id {
                db.counter += 1;
                let new_id = db.counter;
                let status = "open".to_string();
                
                // Update the line
                if marker.line_number <= lines.len() {
                    let line_idx = marker.line_number - 1;
                    let line = lines[line_idx].clone();
                    
                    // Build the new marker
                    let old_marker = format!("{}{}", marker_prefix, marker.marker_type);
                    let new_marker = format!("{}{}:{}:{}", marker_prefix, marker.marker_type, new_id, status);
                    
                    // Replace the marker in the line
                    let new_line = line.replacen(&old_marker, &new_marker, 1);
                    lines[line_idx] = new_line;
                    
                    // Extract content after the marker (from original line)
                    let content_after = line[marker.column_offset + old_marker.len()..].trim();
                    
                    // Add to database
                    new_items.push(DbItem {
                        id: new_id,
                        item_type: marker.marker_type.clone(),
                        file: file_path.clone(),
                        line: marker.line_number,
                        status,
                        content: content_after.to_string(),
                    });
                }
            }
        }
        
        // Write updated content
        let new_content = lines.join("\n");
        if new_content != content {
            fs::write(&full_path, &new_content)
                .with_context(|| format!("Failed to write file: {:?}", full_path))?;
        }
        
        // Add new items to database (in correct order)
        for item in new_items.into_iter().rev() {
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