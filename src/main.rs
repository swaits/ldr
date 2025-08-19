//! LDR - Log, Do, Review
//!
//! A command-line todo application that emphasizes adding items to the top
//! and provides an interactive review mode for processing items.

use clap::{Parser, Subcommand};
use std::io;
use xdg::BaseDirectories;

mod commands;
mod content;
mod input;
mod markdown;
mod migration;

#[cfg(test)]
mod tests;

/// Command-line interface structure for the ldr todo system
#[derive(Parser)]
#[command(name = "ldr")]
#[command(about = "Log, Do, Review - A simple todo system", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for the ldr todo system
#[derive(Subcommand)]
enum Commands {
    /// Add a new item at the top
    #[command(aliases = ["a", "prepend"])]
    Add {
        /// The text to add (e.g., "Read: Book XYZ")
        text: String,
        /// Add as subtask under this task number (e.g., --under 3)
        #[arg(long)]
        under: Option<usize>,
        /// Add to specific list instead of Default
        #[arg(long)]
        list: Option<String>,
    },
    /// List the top N items (default 5)
    #[command(aliases = ["l", "list"])]
    Ls {
        #[arg(short = 'n', long = "num", default_value_t = 5)]
        num: usize,
        /// Show all items (overrides -n/--num)
        #[arg(short = 'a', long = "all")]
        all: bool,
        /// Filter items containing this text (e.g., "read:", "@work")
        filter: Option<String>,
    },
    /// Raise the priority of items (move toward top)
    #[command(aliases = ["u", "prioritize"])]
    Up {
        /// Item references to prioritize (e.g., "1", "2a", "3b")
        refs: Vec<String>,
    },
    /// Archive completed items
    #[command(aliases = ["d", "done", "finish", "check"])]
    Do {
        /// Item references to archive (e.g., "1", "2a", "3b")
        refs: Vec<String>,
    },
    /// Remove items without archiving
    #[command(aliases = ["remove", "delete", "destroy", "forget"])]
    Rm {
        /// Item references to remove (e.g., "1", "2a", "3b")
        refs: Vec<String>,
    },
    /// Edit items in your $EDITOR
    #[command(alias = "e")]
    Edit,
}

/// Entry point that parses CLI arguments and dispatches to appropriate command handlers.
/// Sets up XDG-compliant data directory paths and handles migration from plain text format.
fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let base = BaseDirectories::with_prefix("ldr");
    
    // Old plain text file paths
    let note_path = base
        .place_data_file("note.txt")
        .expect("Failed to create data directory");
    let archive_path = base
        .place_data_file("archive.txt")
        .expect("Failed to create data directory");

    // New Markdown file paths
    let todo_md_path = base
        .place_data_file("todos.md")
        .expect("Failed to create data directory");
    let archive_md_path = base
        .place_data_file("archive.md")
        .expect("Failed to create data directory");

    // Check if migration is needed and perform it
    if migration::needs_migration(&note_path, &archive_path, &todo_md_path, &archive_md_path) {
        if let Err(e) = migration::perform_migration(&note_path, &archive_path, &todo_md_path, &archive_md_path) {
            eprintln!("Migration failed: {}", e);
            return Err(io::Error::other(e));
        }
    }

    match cli.command {
        Commands::Add { text, under, list } => commands::add_entry(&todo_md_path, &text, under, list.as_deref()),
        Commands::Ls { num, all, filter } => {
            commands::list_note(&todo_md_path, num, all, filter.as_deref())
        }
        Commands::Up { refs } => commands::prioritize_items(&todo_md_path, &refs),
        Commands::Do { refs } => commands::archive_items(&todo_md_path, &archive_md_path, &refs),
        Commands::Rm { refs } => commands::remove_items(&todo_md_path, &refs),
        Commands::Edit => commands::edit_note(&todo_md_path),
    }
}
