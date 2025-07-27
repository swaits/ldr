//! Command implementations for the parts todo system.
//!
//! This module contains all the core functionality for managing todo items,
//! including adding, listing, prioritizing, archiving, and interactive review.

use crate::content::{add_entry_to_content, archive_items_in_content, prioritize_items_in_content};
use crate::input::read_key_input;
use std::collections::HashSet;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::Command;
use termion::color;

/// Adds a new entry to the top of the note file.
/// Creates the file if it doesn't exist, otherwise prepends the text to existing content.
pub fn add_entry(path: &Path, text: &str) -> io::Result<()> {
    let mut content = String::new();
    if path.exists() {
        content = fs::read_to_string(path)?;
    }
    let new_content = add_entry_to_content(&content, text);
    fs::write(path, new_content)
}

/// Lists notes from the file with optional filtering and pagination.
/// Displays numbered items with colored output, supports case-insensitive filtering.
pub fn list_note(path: &Path, num: usize, all: bool, filter: Option<&str>) -> io::Result<()> {
    if !path.exists() {
        println!(
            "{}No notes yet.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }
    let file = File::open(path)?;
    let lines: Vec<String> = io::BufReader::new(file).lines().collect::<Result<_, _>>()?;

    let filtered_lines: Vec<(usize, &String)> = if let Some(filter_text) = filter {
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&filter_text.to_lowercase()))
            .collect()
    } else {
        lines.iter().enumerate().collect()
    };

    if filtered_lines.is_empty() {
        if filter.is_some() {
            println!(
                "{}No items found matching filter: \"{}\"{}",
                color::Fg(color::Yellow),
                filter.unwrap(),
                color::Fg(color::Reset)
            );
        } else {
            println!(
                "{}No notes yet.{}",
                color::Fg(color::Yellow),
                color::Fg(color::Reset)
            );
        }
        return Ok(());
    }

    let display_num = if all {
        filtered_lines.len()
    } else {
        num.min(filtered_lines.len())
    };
    for (original_idx, line) in filtered_lines.iter().take(display_num) {
        println!(
            "{}{}. {}{}",
            color::Fg(color::Blue),
            original_idx + 1,
            color::Fg(color::Reset),
            line
        );
    }

    if !all && filter.is_some() && filtered_lines.len() > display_num {
        println!(
            "{}... and {} more matching items{}",
            color::Fg(color::Yellow),
            filtered_lines.len() - display_num,
            color::Fg(color::Reset)
        );
    }

    Ok(())
}

/// Moves specified items to the top of the note file to increase their priority.
/// Takes 1-based item numbers and reorders them to appear at the beginning.
pub fn prioritize_items(note_path: &Path, numbers: &[usize]) -> io::Result<()> {
    if !note_path.exists() {
        println!(
            "{}No notes found.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    let content = fs::read_to_string(note_path)?;
    let lines: Vec<String> = content.lines().map(str::to_string).collect();

    if lines.is_empty() {
        println!(
            "{}No notes to prioritize.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    match prioritize_items_in_content(&lines, numbers) {
        Ok((new_content, prioritized_items)) => {
            fs::write(note_path, new_content)?;
            println!(
                "{}✓ Prioritized {} item(s){}",
                color::Fg(color::Green),
                prioritized_items.len(),
                color::Fg(color::Reset)
            );
            for item in prioritized_items {
                println!(
                    "  {}{}{}",
                    color::Fg(color::Magenta),
                    item,
                    color::Fg(color::Reset)
                );
            }
            Ok(())
        }
        Err(msg) => {
            println!(
                "{}{}{}",
                color::Fg(color::Red),
                msg,
                color::Fg(color::Reset)
            );
            Ok(())
        }
    }
}

/// Removes specified items from the note file and appends them to the archive file.
/// Takes 1-based item numbers, validates them, and handles file I/O operations.
pub fn archive_items(note_path: &Path, archive_path: &Path, numbers: &[usize]) -> io::Result<()> {
    if !note_path.exists() {
        println!(
            "{}No notes found.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    let content = fs::read_to_string(note_path)?;
    let lines: Vec<String> = content.lines().map(str::to_string).collect();

    if lines.is_empty() {
        println!(
            "{}No notes to archive.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    match archive_items_in_content(&lines, numbers) {
        Ok((new_content, archived_items)) => {
            // Archive items
            if !archived_items.is_empty() {
                let mut archive_file = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(archive_path)?;
                for item in &archived_items {
                    writeln!(archive_file, "{item}")?;
                }
            }

            fs::write(note_path, new_content)?;
            println!(
                "{}✓ Archived {} item(s){}",
                color::Fg(color::Green),
                archived_items.len(),
                color::Fg(color::Reset)
            );
            for item in archived_items {
                println!(
                    "  {}{}{}",
                    color::Fg(color::Red),
                    item,
                    color::Fg(color::Reset)
                );
            }
            Ok(())
        }
        Err(msg) => {
            println!(
                "{}{}{}",
                color::Fg(color::Red),
                msg,
                color::Fg(color::Reset)
            );
            Ok(())
        }
    }
}

/// Interactive review mode that presents each item for user decision.
/// Supports prioritizing, archiving, skipping items with keyboard navigation and undo functionality.
pub fn review_note(note_path: &Path, archive_path: &Path) -> io::Result<()> {
    if !note_path.exists() {
        println!("No notes to review.");
        return Ok(());
    }

    let content = fs::read_to_string(note_path)?;
    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    let total = lines.len();

    if total == 0 {
        println!("No notes to review.");
        return Ok(());
    }

    let mut prioritized: Vec<String> = Vec::new();
    let mut to_archive: Vec<String> = Vec::new();
    let mut remove_indices: HashSet<usize> = HashSet::new();
    let mut history: Vec<(usize, String, String)> = Vec::new(); // (index, action, item)

    let mut current_idx = 0;
    let indices: Vec<usize> = (0..total).collect();

    while current_idx < indices.len() {
        let i = indices[current_idx];
        let current = current_idx + 1;

        // Clear screen using ANSI escape sequence
        print!("\x1B[2J\x1B[H");

        println!(
            "{}=== PARTS Review Mode ==={}",
            color::Fg(color::Cyan),
            color::Fg(color::Reset)
        );
        println!(
            "{}Progress: {}/{} items remaining{}",
            color::Fg(color::Yellow),
            current,
            total,
            color::Fg(color::Reset)
        );
        println!();
        println!(
            "{}Item:{} {}",
            color::Fg(color::Green),
            color::Fg(color::Reset),
            lines[i]
        );
        println!();
        println!(
            "{}Actions:{}",
            color::Fg(color::Blue),
            color::Fg(color::Reset)
        );
        println!(
            "  {}[↑/p]{} Prioritize (move to top)",
            color::Fg(color::Magenta),
            color::Fg(color::Reset)
        );
        println!(
            "  {}[↓/a]{} Archive (remove from list)",
            color::Fg(color::Red),
            color::Fg(color::Reset)
        );
        println!(
            "  {}[→/Enter/s]{} Skip (keep in current position)",
            color::Fg(color::White),
            color::Fg(color::Reset)
        );
        println!(
            "  {}[←]{} Go back to previous item",
            color::Fg(color::Cyan),
            color::Fg(color::Reset)
        );
        println!(
            "  {}[q]{} Quit review",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        println!();
        print!(
            "{}Choose action:{} ",
            color::Fg(color::Blue),
            color::Fg(color::Reset)
        );
        io::stdout().flush()?;

        // Read input (handle arrow keys)
        let action = read_key_input()?;

        match action.as_str() {
            "up" | "p" => {
                history.push((i, "prioritize".to_string(), lines[i].clone()));
                prioritized.push(lines[i].clone());
                remove_indices.insert(i);
                println!(
                    "↑\n{}✓ Prioritized{}",
                    color::Fg(color::Magenta),
                    color::Fg(color::Reset)
                );
                current_idx += 1;
            }
            "down" | "a" => {
                history.push((i, "archive".to_string(), lines[i].clone()));
                to_archive.push(lines[i].clone());
                remove_indices.insert(i);
                println!(
                    "↓\n{}✓ Archived{}",
                    color::Fg(color::Red),
                    color::Fg(color::Reset)
                );
                current_idx += 1;
            }
            "right" | "enter" | "s" => {
                history.push((i, "skip".to_string(), lines[i].clone()));
                println!(
                    "→\n{}✓ Skipped{}",
                    color::Fg(color::White),
                    color::Fg(color::Reset)
                );
                current_idx += 1;
            }
            "left" => {
                if let Some((prev_idx, prev_action, prev_item)) = history.pop() {
                    // Undo the previous action
                    match prev_action.as_str() {
                        "prioritize" => {
                            prioritized.retain(|item| item != &prev_item);
                            remove_indices.remove(&prev_idx);
                        }
                        "archive" => {
                            to_archive.retain(|item| item != &prev_item);
                            remove_indices.remove(&prev_idx);
                        }
                        "skip" => {
                            // Nothing to undo for skip
                        }
                        _ => {}
                    }
                    current_idx = current_idx.saturating_sub(1);
                    println!(
                        "←\n{}✓ Went back{}",
                        color::Fg(color::Cyan),
                        color::Fg(color::Reset)
                    );
                } else {
                    println!(
                        "←\n{}No previous item to go back to{}",
                        color::Fg(color::Yellow),
                        color::Fg(color::Reset)
                    );
                }
            }
            "q" => {
                println!(
                    "q\n{}Quitting review and saving changes...{}",
                    color::Fg(color::Yellow),
                    color::Fg(color::Reset)
                );
                break;
            }
            _ => {
                history.push((i, "skip".to_string(), lines[i].clone()));
                println!(
                    "?\n{}✓ Skipped (invalid key){}",
                    color::Fg(color::White),
                    color::Fg(color::Reset)
                );
                current_idx += 1;
            }
        }
    }

    // Clear screen for final summary
    print!("\x1B[2J\x1B[H");
    println!(
        "{}=== Review Complete ==={}",
        color::Fg(color::Green),
        color::Fg(color::Reset)
    );
    println!(
        "{}Prioritized:{} {} items",
        color::Fg(color::Magenta),
        color::Fg(color::Reset),
        prioritized.len()
    );
    println!(
        "{}Archived:{} {} items",
        color::Fg(color::Red),
        color::Fg(color::Reset),
        to_archive.len()
    );
    println!(
        "{}Remaining:{} {} items",
        color::Fg(color::Blue),
        color::Fg(color::Reset),
        total - remove_indices.len()
    );
    println!();

    // Archive items
    if !to_archive.is_empty() {
        let mut archive_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(archive_path)?;
        for arch in to_archive {
            writeln!(archive_file, "{arch}")?;
        }
    }

    // Build remaining items
    let mut remaining: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !remove_indices.contains(&i) {
            remaining.push(line.clone());
        }
    }

    // Combine: prioritized items first, then remaining
    let mut new_lines = prioritized;
    new_lines.extend(remaining);

    // Write back to file
    let new_content = if new_lines.is_empty() {
        String::new()
    } else {
        new_lines.join("\n") + "\n"
    };
    fs::write(note_path, new_content)?;

    println!(
        "{}✓ Changes saved successfully!{}",
        color::Fg(color::Green),
        color::Fg(color::Reset)
    );
    Ok(())
}

/// Opens the note file in the user's preferred editor (from $EDITOR environment variable).
/// Creates the file if it doesn't exist, defaults to nano if $EDITOR is not set.
pub fn edit_note(note_path: &Path) -> io::Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());

    // Create the file if it doesn't exist
    if !note_path.exists() {
        fs::write(note_path, "")?;
    }

    let status = Command::new(&editor).arg(note_path).status()?;

    if !status.success() {
        println!(
            "{}Editor exited with error code: {}{}",
            color::Fg(color::Red),
            status.code().unwrap_or(-1),
            color::Fg(color::Reset)
        );
    }

    Ok(())
}
