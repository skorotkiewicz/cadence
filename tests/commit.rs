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

fn marker_on_line_35() -> String {
    format!("{}   $$fixme avoid duplicate work\n", "\n".repeat(34))
}

#[test]
fn commit_without_staged_files_is_noop() {
    let temp_dir = TempDir::new().unwrap();
    assert!(run_cadence(&temp_dir, &["init"]).status.success());
    fs::write(temp_dir.path().join("main.rs"), "// $$todo test\n").unwrap();
    assert!(run_cadence(&temp_dir, &["add", "main.rs"]).status.success());
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let md_path = temp_dir
        .path()
        .join(".cadence")
        .join("items")
        .join("todo.md");
    fs::write(&md_path, "- [x] $$todo:1:done - test\n").unwrap();

    let output = run_cadence(&temp_dir, &["commit"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Nothing staged"));

    let source = fs::read_to_string(temp_dir.path().join("main.rs")).unwrap();
    assert_eq!(source, "// $$todo:1:open test\n");

    let db = fs::read_to_string(temp_dir.path().join(".cadence").join("db.json")).unwrap();
    assert!(db.contains("\"status\": \"open\""));
}

#[test]
fn commit_applies_markdown_status_only_to_staged_files() {
    let temp_dir = TempDir::new().unwrap();
    assert!(run_cadence(&temp_dir, &["init"]).status.success());
    fs::write(temp_dir.path().join("a.rs"), "// $$todo first\n").unwrap();
    fs::write(temp_dir.path().join("b.rs"), "// $$todo second\n").unwrap();
    assert!(run_cadence(&temp_dir, &["add", "a.rs"]).status.success());
    assert!(run_cadence(&temp_dir, &["add", "b.rs"]).status.success());
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let md_path = temp_dir
        .path()
        .join(".cadence")
        .join("items")
        .join("todo.md");
    fs::write(
        &md_path,
        "- [x] $$todo:1:done - first\n- [x] $$todo:2:done - second\n",
    )
    .unwrap();

    assert!(run_cadence(&temp_dir, &["add", "a.rs"]).status.success());
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let a = fs::read_to_string(temp_dir.path().join("a.rs")).unwrap();
    let b = fs::read_to_string(temp_dir.path().join("b.rs")).unwrap();
    assert_eq!(a, "// $$todo:1:done first\n");
    assert_eq!(b, "// $$todo:2:open second\n");

    let markdown = fs::read_to_string(md_path).unwrap();
    assert!(markdown.contains("- [x] $$todo:1:done - a.rs:1:4 - first"));
    assert!(markdown.contains("- [ ] $$todo:2:open - b.rs:1:4 - second"));
}

#[test]
fn commit_applies_custom_schema_marker_from_markdown() {
    let temp_dir = TempDir::new().unwrap();
    assert!(run_cadence(&temp_dir, &["init"]).status.success());
    fs::write(
        temp_dir.path().join(".cadence").join("schemas.yml"),
        r#"todo:
  statuses: ["open:[ ]", "done:[X]", "in-progress:[~]"]
fixme:
  statuses: ["open:[ ]", "done:[X]", "in-progress:[~]"]
"#,
    )
    .unwrap();
    fs::write(
        temp_dir.path().join("main.rs"),
        "// $$fixme avoid duplicate work\n",
    )
    .unwrap();
    assert!(run_cadence(&temp_dir, &["add", "main.rs"]).status.success());
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let md_path = temp_dir
        .path()
        .join(".cadence")
        .join("items")
        .join("fixme.md");
    fs::write(&md_path, "- [~] $$fixme:1:open - avoid duplicate work\n").unwrap();

    assert!(run_cadence(&temp_dir, &["add", "main.rs"]).status.success());
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let markdown = fs::read_to_string(md_path).unwrap();
    assert_eq!(
        markdown,
        "- [~] $$fixme:1:in-progress - main.rs:1:4 - avoid duplicate work\n"
    );

    let source = fs::read_to_string(temp_dir.path().join("main.rs")).unwrap();
    assert_eq!(source, "// $$fixme:1:in-progress avoid duplicate work\n");
}

#[test]
fn commit_applies_markdown_change_only_to_matching_staged_source_file() {
    let temp_dir = TempDir::new().unwrap();
    assert!(run_cadence(&temp_dir, &["init"]).status.success());
    fs::write(
        temp_dir.path().join(".cadence").join("schemas.yml"),
        r#"fixme:
  statuses: ["open:[ ]", "done:[x]", "in-progress:[~]"]
"#,
    )
    .unwrap();

    fs::create_dir(temp_dir.path().join("tests")).unwrap();
    fs::write(temp_dir.path().join("TEST.md"), marker_on_line_35()).unwrap();
    fs::write(
        temp_dir.path().join("tests").join("TEST.md"),
        marker_on_line_35(),
    )
    .unwrap();

    assert!(run_cadence(&temp_dir, &["add", "TEST.md"]).status.success());
    assert!(
        run_cadence(&temp_dir, &["add", "tests/TEST.md"])
            .status
            .success()
    );
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let md_path = temp_dir
        .path()
        .join(".cadence")
        .join("items")
        .join("fixme.md");
    fs::write(
        &md_path,
        concat!(
            "- [~] $$fixme:1:in-progress - TEST.md:35:4 - avoid duplicate work\n",
            "  asd\n",
            "- [~] $$fixme:2:in-progress - tests/TEST.md:35:4 - avoid duplicate work\n",
        ),
    )
    .unwrap();

    assert!(run_cadence(&temp_dir, &["add", "TEST.md"]).status.success());
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let staged_source = fs::read_to_string(temp_dir.path().join("TEST.md")).unwrap();
    let unstaged_source =
        fs::read_to_string(temp_dir.path().join("tests").join("TEST.md")).unwrap();
    assert!(staged_source.contains("$$fixme:1:in-progress"));
    assert!(unstaged_source.contains("$$fixme:2:open"));

    let markdown = fs::read_to_string(md_path).unwrap();
    assert!(
        markdown
            .contains("- [~] $$fixme:1:in-progress - TEST.md:35:4 - avoid duplicate work\n  asd\n")
    );
    assert!(markdown.contains("- [ ] $$fixme:2:open - tests/TEST.md:35:4 - avoid duplicate work"));
}

#[test]
fn commit_rejects_unknown_custom_schema_marker_in_markdown() {
    let temp_dir = TempDir::new().unwrap();
    assert!(run_cadence(&temp_dir, &["init"]).status.success());
    fs::write(
        temp_dir.path().join(".cadence").join("schemas.yml"),
        r#"fixme:
  statuses: ["open:[ ]", "done:[X]"]
"#,
    )
    .unwrap();
    fs::write(
        temp_dir.path().join("main.rs"),
        "// $$fixme avoid duplicate work\n",
    )
    .unwrap();
    assert!(run_cadence(&temp_dir, &["add", "main.rs"]).status.success());
    assert!(run_cadence(&temp_dir, &["commit"]).status.success());

    let md_path = temp_dir
        .path()
        .join(".cadence")
        .join("items")
        .join("fixme.md");
    fs::write(&md_path, "- [~] $$fixme:1:open - avoid duplicate work\n").unwrap();

    assert!(run_cadence(&temp_dir, &["add", "main.rs"]).status.success());
    let output = run_cadence(&temp_dir, &["commit"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown checklist marker `[~]` for `fixme`"));
}
