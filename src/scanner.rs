use std::fs;
use std::path::Path;

/// Represents a detected marker in a file
#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    pub line_number: usize,
    pub column_offset: usize,
    pub marker_type: String,
    pub has_id: bool,
    pub existing_id: Option<u64>,
    pub existing_status: Option<String>,
}

/// Find all markers in a file
pub fn find_markers(file_path: &Path, marker_prefix: &str) -> anyhow::Result<Vec<Marker>> {
    let content = fs::read_to_string(file_path)?;
    let mut markers = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_markers = find_markers_in_line(line, marker_prefix, line_num + 1);
        markers.extend(line_markers);
    }

    Ok(markers)
}

/// Find markers in a single line
fn find_markers_in_line(line: &str, marker_prefix: &str, line_number: usize) -> Vec<Marker> {
    let mut markers = Vec::new();
    let mut search_start = 0;

    while let Some(pos) = line[search_start..].find(marker_prefix) {
        let abs_pos = search_start + pos;

        // Find the end of the marker type (word characters)
        let rest = &line[abs_pos + marker_prefix.len()..];
        let marker_type: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        if !marker_type.is_empty() {
            // Check if it has the format <prefix>type:id:status
            let after_type = &rest[marker_type.len()..];
            let type_len = marker_type.len();

            if let Some(after_type) = after_type.strip_prefix(':') {
                // Has existing ID
                let parts: Vec<&str> = after_type.splitn(2, ':').collect();
                let existing_id: u64 = parts[0].parse().unwrap_or(0);
                let existing_status = if parts.len() > 1 {
                    // Extract status (until space or end)
                    parts[1].split_whitespace().next().map(|s| s.to_string())
                } else {
                    None
                };

                markers.push(Marker {
                    line_number,
                    column_offset: abs_pos,
                    marker_type: marker_type.clone(),
                    has_id: true,
                    existing_id: Some(existing_id),
                    existing_status,
                });
            } else {
                // New marker without ID
                markers.push(Marker {
                    line_number,
                    column_offset: abs_pos,
                    marker_type: marker_type.clone(),
                    has_id: false,
                    existing_id: None,
                    existing_status: None,
                });
            }

            search_start = abs_pos + marker_prefix.len() + type_len;
        } else {
            search_start = abs_pos + marker_prefix.len();
        }
    }

    markers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_markers_in_line_simple() {
        let markers = find_markers_in_line("// $$todo fix this", "$$", 1);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].marker_type, "todo");
        assert!(!markers[0].has_id);
    }

    #[test]
    fn test_find_markers_in_line_with_id() {
        let markers = find_markers_in_line("// $$todo:5:open fix this", "$$", 1);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].marker_type, "todo");
        assert!(markers[0].has_id);
        assert_eq!(markers[0].existing_id, Some(5));
        assert_eq!(markers[0].existing_status, Some("open".to_string()));
    }

    #[test]
    fn test_find_markers_in_line_multiple() {
        let markers = find_markers_in_line("// $$todo $$fixme", "$$", 1);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].marker_type, "todo");
        assert_eq!(markers[1].marker_type, "fixme");
    }

    #[test]
    fn test_find_markers_in_line_no_marker() {
        let markers = find_markers_in_line("// regular comment", "$$", 1);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_find_markers_preserves_offset() {
        let markers = find_markers_in_line("TODO: $$todo high priority", "$$", 1);
        assert_eq!(markers[0].column_offset, 6);
    }
}
