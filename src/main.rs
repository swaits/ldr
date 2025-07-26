use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::path::Path;
use std::process::Command;
use termion::color;
use termion::raw::IntoRawMode;
use xdg::BaseDirectories;

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(name = "parts")]
#[command(about = "Prepend and Review ToDo System", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

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
        Commands::Add { text } => add_entry(&note_path, &text),
        Commands::Ls { num, all, filter } => list_note(&note_path, num, all, filter.as_deref()),
        Commands::Up { numbers } => prioritize_items(&note_path, &numbers),
        Commands::Do { numbers } => archive_items(&note_path, &archive_path, &numbers),
        Commands::Scan => review_note(&note_path, &archive_path),
        Commands::Edit => edit_note(&note_path),
    }
}

fn add_entry(path: &Path, text: &str) -> io::Result<()> {
    let mut content = String::new();
    if path.exists() {
        content = fs::read_to_string(path)?;
    }
    let new_content = add_entry_to_content(&content, text);
    fs::write(path, new_content)
}

fn add_entry_to_content(existing_content: &str, text: &str) -> String {
    format!("{}\n{}", text, existing_content.trim_end())
}

fn list_note(path: &Path, num: usize, all: bool, filter: Option<&str>) -> io::Result<()> {
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

fn read_key_input() -> io::Result<String> {
    let _stdout = io::stdout().into_raw_mode()?;
    let mut buffer = [0; 3];

    // Read first byte
    io::stdin().read_exact(&mut buffer[0..1])?;

    match buffer[0] {
        27 => {
            // ESC sequence, read next two bytes
            io::stdin().read_exact(&mut buffer[1..3])?;
            match (buffer[1], buffer[2]) {
                (91, 65) => Ok("up".to_string()),    // ESC[A
                (91, 66) => Ok("down".to_string()),  // ESC[B
                (91, 67) => Ok("right".to_string()), // ESC[C
                (91, 68) => Ok("left".to_string()),  // ESC[D
                _ => Ok("unknown".to_string()),
            }
        }
        b'\n' | b'\r' => Ok("enter".to_string()),
        b'q' => Ok("q".to_string()),
        b'p' => Ok("p".to_string()),
        b'a' => Ok("a".to_string()),
        b's' => Ok("s".to_string()),
        _ => Ok("unknown".to_string()),
    }
}

fn prioritize_items(note_path: &Path, numbers: &[usize]) -> io::Result<()> {
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

fn prioritize_items_in_content(
    lines: &[String],
    numbers: &[usize],
) -> Result<(String, Vec<String>), String> {
    // Convert 1-based numbers to 0-based indices and validate
    let mut indices: Vec<usize> = Vec::new();
    for &num in numbers {
        if num == 0 || num > lines.len() {
            return Err(format!(
                "Invalid item number: {}. Valid range: 1-{}",
                num,
                lines.len()
            ));
        }
        indices.push(num - 1);
    }

    // Remove duplicates while preserving order
    let mut seen = HashSet::new();
    indices.retain(|&x| seen.insert(x));

    // Extract items to prioritize in the order specified on command line
    let mut to_prioritize: Vec<String> = Vec::new();
    let indices_set: HashSet<usize> = indices.iter().cloned().collect();

    for &idx in &indices {
        to_prioritize.push(lines[idx].clone());
    }

    // Extract remaining items (preserving original order)
    let mut remaining: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !indices_set.contains(&i) {
            remaining.push(line.clone());
        }
    }

    // Combine: prioritized items first (in command-line order), then remaining
    let mut new_lines = to_prioritize.clone();
    new_lines.extend(remaining);

    let new_content = if new_lines.is_empty() {
        String::new()
    } else {
        new_lines.join("\n") + "\n"
    };

    Ok((new_content, to_prioritize))
}

fn archive_items(note_path: &Path, archive_path: &Path, numbers: &[usize]) -> io::Result<()> {
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

fn archive_items_in_content(
    lines: &[String],
    numbers: &[usize],
) -> Result<(String, Vec<String>), String> {
    // Convert 1-based numbers to 0-based indices and validate
    let mut indices: Vec<usize> = Vec::new();
    for &num in numbers {
        if num == 0 || num > lines.len() {
            return Err(format!(
                "Invalid item number: {}. Valid range: 1-{}",
                num,
                lines.len()
            ));
        }
        indices.push(num - 1);
    }

    // Remove duplicates and sort
    indices.sort_unstable();
    indices.dedup();

    // Extract items to archive and remaining items
    let mut to_archive: Vec<String> = Vec::new();
    let mut remaining: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if indices.contains(&i) {
            to_archive.push(line.clone());
        } else {
            remaining.push(line.clone());
        }
    }

    let new_content = if remaining.is_empty() {
        String::new()
    } else {
        remaining.join("\n") + "\n"
    };

    Ok((new_content, to_archive))
}

fn review_note(note_path: &Path, archive_path: &Path) -> io::Result<()> {
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

fn edit_note(note_path: &Path) -> io::Result<()> {
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
