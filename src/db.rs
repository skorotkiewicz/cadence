use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct StagedFiles {
    pub files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DbItem {
    pub id: u64,
    pub item_type: String,
    pub file: String,
    pub line: usize,
    pub status: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Database {
    pub counter: u64,
    pub items: Vec<DbItem>,
}

pub fn load_staged(dir: &Path) -> Result<StagedFiles> {
    let staged_path = dir.join(".cadence").join("staged.json");
    if !staged_path.exists() {
        return Ok(StagedFiles::default());
    }

    let content = fs::read_to_string(&staged_path)
        .with_context(|| format!("Failed to read: {:?}", staged_path))?;

    let staged: StagedFiles =
        serde_json::from_str(&content).with_context(|| "Failed to parse staged.json")?;

    Ok(staged)
}

pub fn save_staged(dir: &Path, staged: &StagedFiles) -> Result<()> {
    let staged_path = dir.join(".cadence").join("staged.json");
    let content =
        serde_json::to_string_pretty(staged).with_context(|| "Failed to serialize staged")?;

    fs::write(&staged_path, content)
        .with_context(|| format!("Failed to write: {:?}", staged_path))?;

    Ok(())
}

pub fn load_db(dir: &Path) -> Result<Database> {
    let db_path = dir.join(".cadence").join("db.json");
    if !db_path.exists() {
        return Ok(Database::default());
    }

    let content =
        fs::read_to_string(&db_path).with_context(|| format!("Failed to read: {:?}", db_path))?;

    let db: Database = serde_json::from_str(&content).with_context(|| "Failed to parse db.json")?;

    Ok(db)
}

pub fn save_db(dir: &Path, db: &Database) -> Result<()> {
    let db_path = dir.join(".cadence").join("db.json");
    let content = serde_json::to_string_pretty(db).with_context(|| "Failed to serialize db")?;

    fs::write(&db_path, content).with_context(|| format!("Failed to write: {:?}", db_path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_staged_files_default() {
        let staged = StagedFiles::default();
        assert!(staged.files.is_empty());
    }

    #[test]
    fn test_load_staged_returns_default_if_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let result = load_staged(temp_dir.path()).unwrap();
        assert!(result.files.is_empty());
    }

    #[test]
    fn test_save_and_load_staged() {
        let temp_dir = TempDir::new().unwrap();

        // Create .cadence directory first
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();

        let mut staged = StagedFiles::default();
        staged.files.push("src/main.rs".to_string());
        staged.files.push("src/lib.rs".to_string());

        save_staged(temp_dir.path(), &staged).unwrap();
        let loaded = load_staged(temp_dir.path()).unwrap();

        assert_eq!(loaded.files, vec!["src/main.rs", "src/lib.rs"]);
    }

    #[test]
    fn test_db_default() {
        let db = Database::default();
        assert_eq!(db.counter, 0);
        assert!(db.items.is_empty());
    }

    #[test]
    fn test_save_and_load_db() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();

        let mut db = Database {
            counter: 5,
            ..Default::default()
        };
        db.items.push(DbItem {
            id: 1,
            item_type: "todo".to_string(),
            file: "src/main.rs".to_string(),
            line: 10,
            status: "open".to_string(),
            content: "Fix bug".to_string(),
        });

        save_db(temp_dir.path(), &db).unwrap();
        let loaded = load_db(temp_dir.path()).unwrap();

        assert_eq!(loaded.counter, 5);
        assert_eq!(loaded.items.len(), 1);
    }
}
