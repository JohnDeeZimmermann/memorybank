mod cli;
mod commands;
mod config;
mod db;
mod error;
mod graph_ranker;
mod models;
mod output;
mod paths;
mod scorer;
mod sql_log;
mod store;

use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::error::CliResult;
use crate::models::DocumentType;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> CliResult<()> {
    let cli = Cli::parse();
    let root = paths::resolve_root(&cli.root)?;

    match cli.command {
        Commands::Init { rebuild } => commands::init::run(&root, rebuild),
        Commands::Add => {
            let store = store::Store::open_for_write(&root)?;
            commands::add::run(&store)
        }
        Commands::Read { document_id } => {
            paths::require_initialized(&root)?;
            let store = store::Store::open_existing(&root)?;
            commands::read::run(&store, &document_id)
        }
        Commands::QueryFiles {
            files,
            include_invalidated,
        } => {
            paths::require_initialized(&root)?;
            let store = store::Store::open_existing(&root)?;
            commands::query_files::run(&store, &files, include_invalidated)
        }
        Commands::QueryResearch {
            topic,
            include_invalidated,
        } => {
            paths::require_initialized(&root)?;
            let store = store::Store::open_existing(&root)?;
            commands::query_text::run(
                &store,
                "Research",
                DocumentType::Research,
                &topic,
                include_invalidated,
            )
        }
        Commands::QueryPlans {
            term,
            include_invalidated,
        } => {
            paths::require_initialized(&root)?;
            let store = store::Store::open_existing(&root)?;
            commands::query_text::run(
                &store,
                "Plans",
                DocumentType::Plan,
                &term,
                include_invalidated,
            )
        }
    }
}
