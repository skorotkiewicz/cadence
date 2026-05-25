use anyhow::Result;
use cadence::{
    init_cadence, load_staged, save_staged, load_db, save_db,
    update_files_with_ids, generate_markdown_files, parse_markdown_status, update_source_files,
};
use clap::Parser;

fn main() -> Result<()> {
    let cli = cadence::Cli::parse();
    let cwd = std::env::current_dir()?;
    
    match cli.command {
        cadence::Commands::Init => {
            init_cadence(&cwd)?;
            println!("Cadence initialized in .cadence/");
        }
        cadence::Commands::Add { path } => {
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
            let mut db = load_db(&cwd)?;
            
            // Step 1: Update files with new IDs for unmarked markers
            update_files_with_ids(&cwd, &staged.files, &mut db, "$$")?;
            
            // Step 2: Save the updated database
            save_db(&cwd, &db)?;
            
            // Step 3: Generate markdown files
            generate_markdown_files(&cwd, &db)?;
            
            // Step 4: Parse markdown for status changes
            parse_markdown_status(&cwd, &mut db)?;
            
            // Step 5: Save updated database
            save_db(&cwd, &db)?;
            
            // Step 6: Update source files with new statuses
            update_source_files(&cwd, &db)?;
            
            // Step 7: Clear staged files
            staged.files.clear();
            save_staged(&cwd, &staged)?;
            
            println!("Committed {} items", db.items.len());
        }
    }
    
    Ok(())
}