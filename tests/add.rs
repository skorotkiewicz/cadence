use std::fs;
use std::process::{Command, Output};

use tempfile::TempDir;

fn run_cadence(temp_dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cadence"))
        .args(args)
        .current_dir(temp_dir.path())
        .output()
        .unwrap()
}

#[test]
fn add_rejects_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    assert!(run_cadence(&temp_dir, &["init"]).status.success());

    let output = run_cadence(&temp_dir, &["add", "missing.rs"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Path does not exist: missing.rs"));

    let staged = fs::read_to_string(temp_dir.path().join(".cadence").join("staged.json")).unwrap();
    assert!(staged.contains("\"files\": []"));
}

#[test]
fn add_stages_existing_file() {
    let temp_dir = TempDir::new().unwrap();
    assert!(run_cadence(&temp_dir, &["init"]).status.success());
    fs::write(temp_dir.path().join("main.rs"), "// $$todo test\n").unwrap();

    let output = run_cadence(&temp_dir, &["add", "main.rs"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Added: main.rs"));

    let staged = fs::read_to_string(temp_dir.path().join(".cadence").join("staged.json")).unwrap();
    assert!(staged.contains("main.rs"));
}
