//! Parts - A Prepend and Review ToDo System
//!
//! A command-line todo application that emphasizes adding items to the top
//! and provides an interactive review mode for processing items.

use clap::{Parser, Subcommand};
use std::io;
use xdg::BaseDirectories;

mod commands;
mod content;
mod input;

#[cfg(test)]
mod tests;

/// Command-line interface structure for the parts todo system
#[derive(Parser)]
#[command(name = "parts")]
#[command(about = "Prepend and Review ToDo System", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for the parts todo system
#[derive(Subcommand)]
enum Commands {
    /// Add a new item at the top
    #[command(alias = "prepend")]
    Add {
        /// The text to add (e.g., "Read: Book XYZ")
        text: String,
    },
    /// List the top N items (default 5)
    #[command(alias = "list")]
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
    #[command(alias = "prioritize")]
    Up {
        /// Item numbers to prioritize (from "parts ls")
        numbers: Vec<usize>,
    },
    /// Archive completed items
    #[command(aliases = ["done", "finish", "check"])]
    Do {
        /// Item numbers to archive (from "parts ls")
        numbers: Vec<usize>,
    },
    /// Scan and review items interactively, from top to bottom
    #[command(alias = "review")]
    Scan,
    /// Edit items in your $EDITOR
    Edit,
}

/// Entry point that parses CLI arguments and dispatches to appropriate command handlers.
/// Sets up XDG-compliant data directory paths for note.txt and archive.txt files.
fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let base = BaseDirectories::with_prefix("parts");
    let note_path = base
        .place_data_file("note.txt")
        .expect("Failed to create data directory");
    let archive_path = base
        .place_data_file("archive.txt")
        .expect("Failed to create data directory");

    match cli.command {
        Commands::Add { text } => commands::add_entry(&note_path, &text),
        Commands::Ls { num, all, filter } => {
            commands::list_note(&note_path, num, all, filter.as_deref())
        }
        Commands::Up { numbers } => commands::prioritize_items(&note_path, &numbers),
        Commands::Do { numbers } => commands::archive_items(&note_path, &archive_path, &numbers),
        Commands::Scan => commands::review_note(&note_path, &archive_path),
        Commands::Edit => commands::edit_note(&note_path),
    }
}
