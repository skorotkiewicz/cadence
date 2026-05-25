use anyhow::{Result, bail};
use cadence::{
    generate_markdown_files, init_cadence, load_db, load_marker_prefix, load_staged,
    parse_markdown_status_for_files, save_db, save_staged, update_files_with_ids,
    update_source_files_for_files,
};
use clap::Parser;
use std::ffi::OsStr;
use std::path::{Component, Path};

fn main() -> Result<()> {
    let cli = cadence::Cli::parse();
    let cwd = std::env::current_dir()?;

    match cli.command {
        cadence::Commands::Init => {
            init_cadence(&cwd)?;
            println!("Cadence initialized in .cadence/");
        }
        cadence::Commands::Add { path } => {
            if contains_cadence_dir(&path) {
                bail!("Cannot stage .cadence directory or its contents");
            }

            let full_path = cwd.join(&path);
            if !full_path.exists() {
                bail!("Path does not exist: {}", path);
            }
            if !full_path.is_file() {
                bail!("Path is not a file: {}", path);
            }

            let mut staged = load_staged(&cwd)?;
            if !staged.files.contains(&path) {
                staged.files.push(path.clone());
                save_staged(&cwd, &staged)?;
                println!("Added: {}", path);
            } else {
                println!("Already staged: {}", path);
            }
        }
        cadence::Commands::Commit => {
            let mut staged = load_staged(&cwd)?;
            if staged.files.is_empty() {
                println!("Nothing staged");
                return Ok(());
            }

            let mut db = load_db(&cwd)?;
            let marker_prefix = load_marker_prefix(&cwd)?;
            let staged_source_files = staged_source_files(&staged.files);

            // Step 1: Parse markdown for status changes FIRST (to get user edits)
            parse_markdown_status_for_files(&cwd, &mut db, &marker_prefix, &staged_source_files)?;

            // Step 2: Update source files with those status changes
            update_source_files_for_files(&cwd, &db, &marker_prefix, &staged_source_files)?;

            // Step 3: Update files with new IDs for unmarked markers
            update_files_with_ids(&cwd, &staged_source_files, &mut db, &marker_prefix)?;

            // Step 4: Save the updated database
            save_db(&cwd, &db)?;

            // Step 5: Generate markdown files with updated data
            generate_markdown_files(&cwd, &db, &marker_prefix)?;

            // Step 6: Clear staged files
            let committed_items = db
                .items
                .iter()
                .filter(|item| staged_source_files.contains(&item.file))
                .count();
            staged.files.clear();
            save_staged(&cwd, &staged)?;

            println!("Committed {} items", committed_items);
        }
        cadence::Commands::Reset => {
            let mut staged = load_staged(&cwd)?;
            let count = staged.files.len();
            staged.files.clear();
            save_staged(&cwd, &staged)?;
            println!("Unstaged {} files", count);
        }
    }

    Ok(())
}

fn staged_source_files(files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter(|file| !contains_cadence_dir(file))
        .cloned()
        .collect()
}

fn contains_cadence_dir(path: &str) -> bool {
    Path::new(path).components().any(
        |component| matches!(component, Component::Normal(name) if name == OsStr::new(".cadence")),
    )
}
