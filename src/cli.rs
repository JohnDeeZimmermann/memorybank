use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "memorybank")]
#[command(about = "Agent-friendly semantic memory store for a codebase")]
pub struct Cli {
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init {
        #[arg(long)]
        rebuild: bool,
    },
    Add,
    Read {
        document_id: String,
    },
    QueryFiles {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(long)]
        include_invalidated: bool,
    },
    QueryResearch {
        topic: String,
        #[arg(long)]
        include_invalidated: bool,
    },
    QueryPlans {
        term: String,
        #[arg(long)]
        include_invalidated: bool,
    },
}
