use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::DEFAULT_MARKER_PREFIX;

pub fn init_cadence(dir: &Path) -> Result<()> {
    let cadence_dir = dir.join(".cadence");

    // Create .cadence directory
    fs::create_dir_all(&cadence_dir)
        .with_context(|| format!("Failed to create directory: {:?}", cadence_dir))?;

    // Create config.yml
    let config_content = format!(
        "# Cadence configuration\nmarker_prefix: \"{}\"\n",
        DEFAULT_MARKER_PREFIX
    );
    fs::write(cadence_dir.join("config.yml"), config_content)
        .with_context(|| "Failed to write config.yml")?;

    // Create schemas.yml with default schemas
    let schemas_content = r#"# Marker schemas
todo:
  statuses: ["open:[ ]", "done:[x]", "in-progress:[~]"]
fixme:
  statuses: ["open:[ ]", "done:[x]"]
hack:
  statuses: ["open:[ ]", "done:[x]"]
"#;
    fs::write(cadence_dir.join("schemas.yml"), schemas_content)
        .with_context(|| "Failed to write schemas.yml")?;

    // Create empty db.json
    let db_content = r#"{"counter": 0, "items": []}"#;
    fs::write(cadence_dir.join("db.json"), db_content)
        .with_context(|| "Failed to write db.json")?;

    // Create empty staged.json
    let staged_content = r#"{"files": []}"#;
    fs::write(cadence_dir.join("staged.json"), staged_content)
        .with_context(|| "Failed to write staged.json")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_cadence_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = init_cadence(temp_dir.path());
        assert!(result.is_ok());

        let cadence_dir = temp_dir.path().join(".cadence");
        assert!(cadence_dir.exists());
    }

    #[test]
    fn test_init_creates_config_yml() {
        let temp_dir = TempDir::new().unwrap();
        init_cadence(temp_dir.path()).unwrap();

        let config_path = temp_dir.path().join(".cadence").join("config.yml");
        assert!(config_path.exists());

        let content = fs::read_to_string(config_path).unwrap();
        assert!(content.contains("marker_prefix"));
    }

    #[test]
    fn test_init_creates_schemas_yml() {
        let temp_dir = TempDir::new().unwrap();
        init_cadence(temp_dir.path()).unwrap();

        let schemas_path = temp_dir.path().join(".cadence").join("schemas.yml");
        assert!(schemas_path.exists());

        let content = fs::read_to_string(schemas_path).unwrap();
        assert!(content.contains("todo:"));
        assert!(content.contains("fixme:"));
    }

    #[test]
    fn test_init_creates_db_json() {
        let temp_dir = TempDir::new().unwrap();
        init_cadence(temp_dir.path()).unwrap();

        let db_path = temp_dir.path().join(".cadence").join("db.json");
        assert!(db_path.exists());

        let content = fs::read_to_string(db_path).unwrap();
        assert!(content.contains("\"counter\": 0"));
        assert!(content.contains("\"items\""));
    }

    #[test]
    fn test_init_creates_staged_json() {
        let temp_dir = TempDir::new().unwrap();
        init_cadence(temp_dir.path()).unwrap();

        let staged_path = temp_dir.path().join(".cadence").join("staged.json");
        assert!(staged_path.exists());

        let content = fs::read_to_string(staged_path).unwrap();
        assert!(content.contains("\"files\""));
    }
}
