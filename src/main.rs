use anyhow::{Context, Result, bail};
use cadence::{
    generate_markdown_files, init_cadence, load_db, load_marker_prefix, load_staged,
    parse_markdown_status_for_files, save_db, save_staged, update_files_with_ids,
    update_source_files_for_files,
};
use clap::Parser;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

fn main() -> Result<()> {
    let cli = cadence::Cli::parse();
    let cwd = std::env::current_dir()?;

    match cli.command {
        cadence::Commands::Init => {
            init_cadence(&cwd)?;
            println!("Cadence initialized in .cadence/");
        }
        cadence::Commands::Add { path } => {
            if contains_cadence_dir(Path::new(&path)) {
                bail!("Cannot stage .cadence directory or its contents");
            }

            let stage_files = stage_files_for_path(&cwd, &path)?;
            let is_single_file = cwd.join(&path).is_file();

            let mut staged = load_staged(&cwd)?;
            let mut added = 0;
            for file in &stage_files {
                if !staged.files.contains(file) {
                    staged.files.push(file.clone());
                    added += 1;
                }
            }

            if added > 0 {
                save_staged(&cwd, &staged)?;
            }

            if is_single_file {
                if added > 0 {
                    println!("Added: {}", stage_files[0]);
                } else {
                    println!("Already staged: {}", stage_files[0]);
                }
            } else if added > 0 {
                println!("Added {} files", added);
            } else {
                println!("No new files added");
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
        .filter(|file| !contains_cadence_dir(Path::new(file)))
        .cloned()
        .collect()
}

fn stage_files_for_path(cwd: &Path, path: &str) -> Result<Vec<String>> {
    let full_path = cwd.join(path);
    if !full_path.exists() {
        bail!("Path does not exist: {}", path);
    }

    if full_path.is_file() {
        return Ok(vec![stage_path_string(Path::new(path))]);
    }

    if full_path.is_dir() {
        let mut files = Vec::new();
        collect_stage_files(cwd, &full_path, &mut files)?;
        files.sort();
        return Ok(files);
    }

    bail!("Path is not a file or directory: {}", path);
}

fn collect_stage_files(cwd: &Path, dir: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read directory: {:?}", dir))?
    {
        let entry = entry.with_context(|| format!("Failed to read directory entry: {:?}", dir))?;
        let path = entry.path();
        if contains_cadence_dir(&path) {
            continue;
        }

        let file_type = entry
            .file_type()
            .with_context(|| format!("Failed to read file type: {:?}", path))?;
        if file_type.is_dir() {
            collect_stage_files(cwd, &path, files)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(cwd)
                .with_context(|| format!("Failed to make path relative: {:?}", path))?;
            files.push(stage_path_string(relative));
        }
    }

    Ok(())
}

fn stage_path_string(path: &Path) -> String {
    if path.is_absolute() {
        return path.to_string_lossy().replace('\\', "/");
    }

    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(PathBuf::from(value)),
            Component::CurDir => None,
            Component::ParentDir => Some(PathBuf::from("..")),
            _ => None,
        })
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/")
}

fn contains_cadence_dir(path: &Path) -> bool {
    path.components().any(
        |component| matches!(component, Component::Normal(name) if name == OsStr::new(".cadence")),
    )
}
