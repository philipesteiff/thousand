use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "thousand", version, about = "Thousand v1 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Find {
        #[arg(long)]
        config: PathBuf,
    },
    Solve {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        issue: String,
    },
    Validate {
        #[arg(long)]
        config: PathBuf,
    },
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Find { config } => thousand::run::find(&config).await,
        Commands::Solve { config, issue } => thousand::run::solve(&config, &issue).await,
        Commands::Validate { config } => {
            let cfg = thousand::config::WorkflowConfig::load(&config)?;
            cfg.validate()?;
            println!("config ok: {}", config.display());
            Ok(())
        }
        Commands::Version => {
            println!("thousand {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
