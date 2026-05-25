use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cadence")]
#[command(about = "Track markers in source files")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Add { path: String },
    Commit,
    Reset,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_init_command() {
        let args = vec!["cadence", "init"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Init => (),
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_parse_add_command() {
        let args = vec!["cadence", "add", "src/main.rs"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Add { path } => assert_eq!(path, "src/main.rs"),
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_parse_commit_command() {
        let args = vec!["cadence", "commit"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Commit => (),
            _ => panic!("Expected Commit command"),
        }
    }

    #[test]
    fn test_cli_parse_reset_command() {
        let args = vec!["cadence", "reset"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Reset => (),
            _ => panic!("Expected Reset command"),
        }
    }
}