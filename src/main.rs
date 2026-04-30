mod cli;
mod commands;
mod db;
mod error;
mod models;
mod output;
mod paths;
mod sql_log;

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
        Commands::Add => commands::add::run(&root),
        Commands::Read { document_id } => commands::read::run(&root, &document_id),
        Commands::QueryFiles {
            files,
            include_invalidated,
        } => commands::query_files::run(&root, &files, include_invalidated),
        Commands::QueryResearch {
            topic,
            include_invalidated,
        } => commands::query_text::run(
            &root,
            "Research",
            DocumentType::Research,
            &topic,
            include_invalidated,
        ),
        Commands::QueryPlans {
            term,
            include_invalidated,
        } => commands::query_text::run(
            &root,
            "Plans",
            DocumentType::Plan,
            &term,
            include_invalidated,
        ),
    }
}
