use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

pub const DEFAULT_MARKER_PREFIX: &str = "$$";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CadenceConfig {
    pub marker_prefix: String,
}

impl Default for CadenceConfig {
    fn default() -> Self {
        Self {
            marker_prefix: DEFAULT_MARKER_PREFIX.to_string(),
        }
    }
}

pub fn load_config(dir: &Path) -> Result<CadenceConfig> {
    let config_path = dir.join(".cadence").join("config.yml");
    if !config_path.exists() {
        return Ok(CadenceConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read: {:?}", config_path))?;
    let mut config = CadenceConfig::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(value) = line.strip_prefix("marker_prefix:") {
            config.marker_prefix = parse_marker_prefix(value.trim())?;
        }
    }

    Ok(config)
}

pub fn load_marker_prefix(dir: &Path) -> Result<String> {
    Ok(load_config(dir)?.marker_prefix)
}

fn parse_marker_prefix(value: &str) -> Result<String> {
    let prefix = if let Some(value) = value.strip_prefix('"') {
        let Some((prefix, _)) = value.split_once('"') else {
            bail!("Invalid marker_prefix: missing closing double quote");
        };
        prefix
    } else if let Some(value) = value.strip_prefix('\'') {
        let Some((prefix, _)) = value.split_once('\'') else {
            bail!("Invalid marker_prefix: missing closing single quote");
        };
        prefix
    } else {
        value.split('#').next().unwrap_or(value).trim()
    };

    if prefix.is_empty() {
        bail!("marker_prefix cannot be empty");
    }

    Ok(prefix.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_config_defaults_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let config = load_config(temp_dir.path()).unwrap();

        assert_eq!(config.marker_prefix, DEFAULT_MARKER_PREFIX);
    }

    #[test]
    fn test_load_config_reads_quoted_marker_prefix() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();
        fs::write(
            temp_dir.path().join(".cadence").join("config.yml"),
            "marker_prefix: \"@@\"\n",
        )
        .unwrap();

        let config = load_config(temp_dir.path()).unwrap();

        assert_eq!(config.marker_prefix, "@@");
    }

    #[test]
    fn test_load_config_reads_unquoted_marker_prefix() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();
        fs::write(
            temp_dir.path().join(".cadence").join("config.yml"),
            "marker_prefix: @@ # comment\n",
        )
        .unwrap();

        let config = load_config(temp_dir.path()).unwrap();

        assert_eq!(config.marker_prefix, "@@");
    }

    #[test]
    fn test_load_config_rejects_empty_marker_prefix() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".cadence")).unwrap();
        fs::write(
            temp_dir.path().join(".cadence").join("config.yml"),
            "marker_prefix: \"\"\n",
        )
        .unwrap();

        let err = load_config(temp_dir.path()).unwrap_err();

        assert!(err.to_string().contains("marker_prefix cannot be empty"));
    }
}
