//! Command implementations for the ldr todo system with Markdown support.
//!
//! This module contains all the core functionality for managing todo items,
//! including adding, listing, prioritizing, archiving, and editing.
//! Now supports subtasks and multiple lists in Markdown format.

use crate::markdown::{
    generate_archive_file, generate_todo_file, parse_archive_file, parse_todo_file, ArchiveFile,
    Task, TaskRef, TodoFile,
};
use anyhow::{anyhow, Context, Result};
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use termion::color;

// Custom 256-color support
struct Color256(u8);

impl fmt::Display for Color256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1b[38;5;{}m", self.0)
    }
}

// HSV to RGB conversion
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if (0.0..1.0).contains(&h) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

// Convert RGB to closest 256-color palette index
fn rgb_to_256_color(r: u8, g: u8, b: u8) -> u8 {
    // For colors 16-231: 6x6x6 color cube
    // Each component ranges from 0-5, mapped from 0-255
    let r_index = if r < 48 {
        0
    } else if r < 115 {
        1
    } else {
        (r - 55) / 40
    };
    let g_index = if g < 48 {
        0
    } else if g < 115 {
        1
    } else {
        (g - 55) / 40
    };
    let b_index = if b < 48 {
        0
    } else if b < 115 {
        1
    } else {
        (b - 55) / 40
    };

    16 + 36 * r_index + 6 * g_index + b_index
}

// Generate color from HSV values
fn hsv_color(h: f32, s: f32, v: f32) -> Color256 {
    let (r, g, b) = hsv_to_rgb(h, s, v);
    Color256(rgb_to_256_color(r, g, b))
}

// Color scheme configuration
struct ColorScheme {
    // Main task colors (bright, high saturation)
    task1_hue: f32, // Primary hue for odd tasks
    task2_hue: f32, // Primary hue for even tasks
    main_saturation: f32,
    main_value: f32,

    // Subtask adjustments
    value_reduction: f32, // Amount to reduce brightness for subtasks
}

impl ColorScheme {
    fn new() -> Self {
        if Self::is_dark_terminal() {
            // Dark terminal scheme - bright colors
            ColorScheme {
                task1_hue: 200.0, // Light cyan-blue
                task2_hue: 40.0,  // Light desert tan/gold
                main_saturation: 0.7,
                main_value: 0.95, // Very bright
                value_reduction: 0.2,
            }
        } else {
            // Light terminal scheme - darker colors
            ColorScheme {
                task1_hue: 210.0, // Darker blue
                task2_hue: 30.0,  // Darker orange
                main_saturation: 0.8,
                main_value: 0.6, // Much darker for light backgrounds
                value_reduction: 0.15,
            }
        }
    }

    fn is_dark_terminal() -> bool {
        // Check various indicators for dark terminal

        // Check COLORFGBG environment variable (format: "15;0" means white fg, black bg)
        if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
            if let Some(bg) = colorfgbg.split(';').nth(1) {
                if let Ok(bg_color) = bg.parse::<u8>() {
                    // Background colors 0-7 are typically dark
                    return bg_color < 8;
                }
            }
        }

        // Check for common dark terminal themes in environment
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("dark") {
                return true;
            }
        }

        // Check common terminal emulators that default to dark
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            match term_program.as_str() {
                "iTerm.app" | "WezTerm" | "Alacritty" | "ghostty" => return true,
                _ => {}
            }
        }

        // Default to dark terminal (most developers use dark themes)
        true
    }

    fn get_main_task_color(&self, task_num: usize) -> Color256 {
        let hue = if task_num % 2 == 1 {
            self.task1_hue
        } else {
            self.task2_hue
        };
        hsv_color(hue, self.main_saturation, self.main_value)
    }

    fn get_subtask_color(&self, task_num: usize, _subtask_idx: usize) -> Color256 {
        let base_hue = if task_num % 2 == 1 {
            self.task1_hue
        } else {
            self.task2_hue
        };

        // Simply inherit parent color but reduce saturation and value
        let reduced_saturation = self.main_saturation - 0.15; // Reduce saturation by 15%
        let reduced_value = self.main_value - self.value_reduction;

        hsv_color(base_hue, reduced_saturation, reduced_value)
    }
}

/// Adds a new entry to the todo file.
/// Creates the file if it doesn't exist, otherwise prepends to the main list.
/// Can add as subtask if `under` is specified.
pub fn add_entry(path: &Path, text: &str, under: Option<usize>) -> Result<()> {
    // Validate input
    if text.trim().is_empty() {
        return Err(anyhow!("Cannot add empty task"));
    }

    // Limit task text length to prevent abuse
    const MAX_TASK_LENGTH: usize = 500;
    if text.len() > MAX_TASK_LENGTH {
        return Err(anyhow!(
            "Task text too long ({}). Maximum length is {} characters",
            text.len(),
            MAX_TASK_LENGTH
        ));
    }
    let mut todo_file = if path.exists() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        parse_todo_file(&content).map_err(|e| anyhow!("Failed to parse file: {}", e))?
    } else {
        TodoFile::new("TODOs".to_string())
    };

    if let Some(task_num) = under {
        // Add as subtask
        if task_num == 0 || task_num > todo_file.tasks.len() {
            return Err(anyhow!(
                "Invalid task number: {}. Valid range: 1-{}",
                task_num,
                todo_file.tasks.len()
            ));
        }

        // Limit number of subtasks per task
        const MAX_SUBTASKS: usize = 26; // a-z
        let task = &todo_file.tasks[task_num - 1];
        if task.subtasks.len() >= MAX_SUBTASKS {
            return Err(anyhow!(
                "Task {} already has maximum number of subtasks ({})",
                task_num,
                MAX_SUBTASKS
            ));
        }

        todo_file.tasks[task_num - 1].add_subtask(text.to_string());
        println!(
            "{}✓ Added subtask to task {}: {}{}",
            color::Fg(color::Green),
            task_num,
            text,
            color::Fg(color::Reset)
        );
    } else {
        // Add as new main task at top
        // Limit total number of tasks to prevent abuse
        const MAX_TASKS: usize = 1000;
        if todo_file.tasks.len() >= MAX_TASKS {
            return Err(anyhow!(
                "Maximum number of tasks ({}) reached. Please archive or remove some tasks first",
                MAX_TASKS
            ));
        }

        let task = Task::new(text.to_string());
        todo_file.prepend_task(task);
        println!(
            "{}✓ Added: {}{}",
            color::Fg(color::Green),
            text,
            color::Fg(color::Reset)
        );
    }

    let content = generate_todo_file(&todo_file);
    fs::write(path, content).with_context(|| format!("Failed to write file: {}", path.display()))
}

/// Lists tasks with numbered display including subtasks.
/// Displays task numbers and subtask letters, supports filtering.
pub fn list_note(path: &Path, num: usize, all: bool, filter: Option<&str>) -> Result<()> {
    if !path.exists() {
        println!(
            "{}No notes yet.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    let todo_file =
        parse_todo_file(&content).map_err(|e| anyhow!("Failed to parse file: {}", e))?;

    if todo_file.is_empty() {
        println!(
            "{}No notes yet.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    // Build list of items for filtering and display
    let display_items: Vec<_> = if let Some(filter_text) = filter {
        let mut filtered = Vec::new();
        let filter_lower = filter_text.to_lowercase();

        for (task_idx, task) in todo_file.tasks.iter().enumerate() {
            let task_num = task_idx + 1;
            let task_matches = task.text.to_lowercase().contains(&filter_lower);

            // Check which subtasks match
            let mut matching_subtasks = Vec::new();
            for (subtask_idx, subtask) in task.subtasks.iter().enumerate() {
                if subtask.to_lowercase().contains(&filter_lower) {
                    matching_subtasks.push(subtask_idx);
                }
            }

            if task_matches {
                // If task matches, include task and ALL its subtasks
                let task_line = format!("{:3}. {}", task_num, task.text);
                filtered.push((task_num, None, task_line));

                for (subtask_idx, subtask) in task.subtasks.iter().enumerate() {
                    let letter = (b'a' + subtask_idx as u8) as char;
                    let subtask_line = format!("     {}. {}", letter, subtask);
                    filtered.push((task_num, Some(subtask_idx), subtask_line));
                }
            } else if !matching_subtasks.is_empty() {
                // If only subtasks match, include task and only matching subtasks
                let task_line = format!("{:3}. {}", task_num, task.text);
                filtered.push((task_num, None, task_line));

                for &subtask_idx in &matching_subtasks {
                    let letter = (b'a' + subtask_idx as u8) as char;
                    let subtask_line = format!("     {}. {}", letter, &task.subtasks[subtask_idx]);
                    filtered.push((task_num, Some(subtask_idx), subtask_line));
                }
            }
        }

        filtered
    } else {
        // No filter - include everything
        let mut all_items = Vec::new();
        for (task_idx, task) in todo_file.tasks.iter().enumerate() {
            let task_num = task_idx + 1;
            let task_line = format!("{:3}. {}", task_num, task.text);
            all_items.push((task_num, None, task_line));

            // Add subtasks if any
            for (subtask_idx, subtask) in task.subtasks.iter().enumerate() {
                let letter = (b'a' + subtask_idx as u8) as char;
                let subtask_line = format!("     {}. {}", letter, subtask);
                all_items.push((task_num, Some(subtask_idx), subtask_line));
            }
        }
        all_items
    };

    if display_items.is_empty() {
        if filter.is_some() {
            println!(
                "{}No items found matching filter: \"{}\"{}",
                color::Fg(color::Yellow),
                filter.unwrap_or(""),
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

    let display_count = if all {
        display_items.len()
    } else {
        num.min(display_items.len())
    };

    let color_scheme = ColorScheme::new();

    for (task_num, subtask_idx, line) in display_items.iter().take(display_count) {
        if subtask_idx.is_none() {
            // Main task - use HSV-based bright colors
            let color = color_scheme.get_main_task_color(*task_num);
            println!("{}{}{}", color, line, color::Fg(color::Reset));
        } else {
            // Subtask - use same color family as parent but dimmer
            let color = color_scheme.get_subtask_color(*task_num, subtask_idx.unwrap());
            println!("{}{}{}", color, line, color::Fg(color::Reset));
        }
    }

    if !all && display_items.len() > display_count {
        println!(
            "{}... and {} more items{}",
            color::Fg(color::Yellow),
            display_items.len() - display_count,
            color::Fg(color::Reset)
        );
    }

    Ok(())
}

/// Parse task references and perform operations on tasks/subtasks
pub fn prioritize_items(todo_path: &Path, refs: &[String]) -> Result<()> {
    if !todo_path.exists() {
        println!(
            "{}No notes found.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    let content = fs::read_to_string(todo_path)
        .with_context(|| format!("Failed to read file: {}", todo_path.display()))?;
    let mut todo_file =
        parse_todo_file(&content).map_err(|e| anyhow!("Failed to parse file: {}", e))?;

    if todo_file.is_empty() {
        println!(
            "{}No notes found.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    // Parse task references
    let mut task_refs = Vec::new();
    for ref_str in refs {
        match TaskRef::parse(ref_str) {
            Ok(task_ref) => {
                if task_ref.task_index >= todo_file.tasks.len() {
                    println!(
                        "{}Invalid task number: {}. Valid range: 1-{}{}",
                        color::Fg(color::Red),
                        task_ref.task_index + 1,
                        todo_file.tasks.len(),
                        color::Fg(color::Reset)
                    );
                    return Ok(());
                }

                if let Some(subtask_idx) = task_ref.subtask_index {
                    let task = &todo_file.tasks[task_ref.task_index];
                    if subtask_idx >= task.subtasks.len() {
                        println!(
                            "{}Invalid subtask: {}{}. Task {} has {} subtasks{}",
                            color::Fg(color::Red),
                            ref_str,
                            color::Fg(color::Reset),
                            task_ref.task_index + 1,
                            task.subtasks.len(),
                            color::Fg(color::Reset)
                        );
                        return Ok(());
                    }
                }

                task_refs.push(task_ref);
            }
            Err(e) => {
                println!(
                    "{}Invalid task reference '{}': {}{}",
                    color::Fg(color::Red),
                    ref_str,
                    e,
                    color::Fg(color::Reset)
                );
                return Ok(());
            }
        }
    }

    // For prioritizing, we move entire tasks to the top (subtask refs move their parent task)
    let mut tasks_to_move = Vec::new();
    let mut moved_task_indices = HashSet::new();

    for task_ref in &task_refs {
        if !moved_task_indices.contains(&task_ref.task_index) {
            tasks_to_move.push(task_ref.task_index);
            moved_task_indices.insert(task_ref.task_index);
        }
    }

    // Create new task order by swapping moved tasks to front
    let old_tasks = std::mem::take(&mut todo_file.tasks);
    let mut new_tasks = Vec::with_capacity(old_tasks.len());
    let mut moved_task_names = Vec::new();

    // First add the moved tasks in the order specified
    for &task_idx in &tasks_to_move {
        if task_idx < old_tasks.len() {
            new_tasks.push(old_tasks[task_idx].clone());
            moved_task_names.push(old_tasks[task_idx].text.clone());
        }
    }

    // Then add all non-moved tasks
    for (idx, task) in old_tasks.into_iter().enumerate() {
        if !moved_task_indices.contains(&idx) {
            new_tasks.push(task);
        }
    }

    todo_file.tasks = new_tasks;

    let new_content = generate_todo_file(&todo_file);
    fs::write(todo_path, new_content)
        .with_context(|| format!("Failed to write file: {}", todo_path.display()))?;

    println!(
        "{}✓ Prioritized {} task(s){}",
        color::Fg(color::Green),
        moved_task_names.len(),
        color::Fg(color::Reset)
    );

    for task_name in moved_task_names {
        println!(
            "  {}{}{}",
            color::Fg(color::Magenta),
            task_name,
            color::Fg(color::Reset)
        );
    }

    Ok(())
}

/// Internal helper to process items for removal or archiving
fn process_items_for_removal(
    todo_path: &Path,
    refs: &[String],
    archive_path: Option<&Path>,
) -> Result<()> {
    let should_archive = archive_path.is_some();
    if !todo_path.exists() {
        println!(
            "{}No notes found.{}",
            color::Fg(color::Yellow),
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    let content = fs::read_to_string(todo_path)
        .with_context(|| format!("Failed to read file: {}", todo_path.display()))?;
    let mut todo_file =
        parse_todo_file(&content).map_err(|e| anyhow!("Failed to parse file: {}", e))?;

    if todo_file.is_empty() {
        let action = if should_archive { "archive" } else { "remove" };
        println!(
            "{}No notes to {}.{}",
            color::Fg(color::Yellow),
            action,
            color::Fg(color::Reset)
        );
        return Ok(());
    }

    // Parse task references
    let mut task_refs = Vec::new();
    for ref_str in refs {
        match TaskRef::parse(ref_str) {
            Ok(task_ref) => task_refs.push((ref_str.clone(), task_ref)),
            Err(e) => {
                println!(
                    "{}Invalid task reference '{}': {}{}",
                    color::Fg(color::Red),
                    ref_str,
                    e,
                    color::Fg(color::Reset)
                );
                return Ok(());
            }
        }
    }

    // Separate tasks and subtasks to archive
    let mut tasks_to_archive = Vec::new();
    let mut subtasks_to_remove = Vec::new(); // (task_idx, subtask_idx)
    let mut whole_tasks_to_remove = HashSet::new();

    for (ref_str, task_ref) in &task_refs {
        if task_ref.task_index >= todo_file.tasks.len() {
            println!(
                "{}Invalid task number in '{}': {}. Valid range: 1-{}{}",
                color::Fg(color::Red),
                ref_str,
                task_ref.task_index + 1,
                todo_file.tasks.len(),
                color::Fg(color::Reset)
            );
            return Ok(());
        }

        if let Some(subtask_idx) = task_ref.subtask_index {
            // Archiving a subtask
            let task = &todo_file.tasks[task_ref.task_index];
            if subtask_idx >= task.subtasks.len() {
                println!(
                    "{}Invalid subtask '{}': Task {} has {} subtasks{}",
                    color::Fg(color::Red),
                    ref_str,
                    task_ref.task_index + 1,
                    task.subtasks.len(),
                    color::Fg(color::Reset)
                );
                return Ok(());
            }
            subtasks_to_remove.push((task_ref.task_index, subtask_idx));
        } else {
            // Archiving whole task
            whole_tasks_to_remove.insert(task_ref.task_index);
        }
    }

    // Collect items to archive
    for &task_idx in &whole_tasks_to_remove {
        tasks_to_archive.push(todo_file.tasks[task_idx].clone());
    }

    for &(task_idx, subtask_idx) in &subtasks_to_remove {
        if !whole_tasks_to_remove.contains(&task_idx) {
            let subtask_text = todo_file.tasks[task_idx].subtasks[subtask_idx].clone();
            tasks_to_archive.push(Task::new(subtask_text));
        }
    }

    // Load archive file if we're archiving
    let mut archive_file = if let Some(archive_path) = archive_path {
        if archive_path.exists() {
            let archive_content = fs::read_to_string(archive_path)
                .with_context(|| format!("Failed to read archive: {}", archive_path.display()))?;
            parse_archive_file(&archive_content)
                .map_err(|e| anyhow!("Failed to parse file: {}", e))?
        } else {
            ArchiveFile::new()
        }
    } else {
        ArchiveFile::new()
    };

    // Add items to archive if we're archiving
    if should_archive && !tasks_to_archive.is_empty() {
        if let Some(archive_path) = archive_path {
            archive_file.add_items_for_today("Default", tasks_to_archive.clone());
            let archive_content = generate_archive_file(&archive_file);
            fs::write(archive_path, archive_content)
                .with_context(|| format!("Failed to write archive: {}", archive_path.display()))?;
        }
    }

    // Remove items from todo file
    // Remove subtasks first (in reverse order to maintain indices)
    let mut subtasks_by_task: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();
    for &(task_idx, subtask_idx) in &subtasks_to_remove {
        if !whole_tasks_to_remove.contains(&task_idx) {
            subtasks_by_task
                .entry(task_idx)
                .or_default()
                .push(subtask_idx);
        }
    }

    // Track tasks that might need auto-completion
    let mut tasks_to_auto_complete = Vec::new();

    for (task_idx, mut subtask_indices) in subtasks_by_task {
        subtask_indices.sort_by(|a, b| b.cmp(a)); // Sort in reverse order
        for subtask_idx in subtask_indices {
            todo_file.tasks[task_idx].subtasks.remove(subtask_idx);
        }

        // Check if this task now has no subtasks left and should be auto-completed
        if todo_file.tasks[task_idx].subtasks.is_empty() {
            tasks_to_auto_complete.push(task_idx);
        }
    }

    // Auto-complete parent tasks that have no subtasks left
    let mut auto_completed_tasks = Vec::new();
    if !tasks_to_auto_complete.is_empty() {
        for &task_idx in &tasks_to_auto_complete {
            auto_completed_tasks.push(todo_file.tasks[task_idx].clone());
        }

        // Add auto-completed tasks to archive if we're archiving
        if should_archive && !auto_completed_tasks.is_empty() {
            if let Some(archive_path) = archive_path {
                archive_file.add_items_for_today("Default", auto_completed_tasks.clone());
                let archive_content = generate_archive_file(&archive_file);
                fs::write(archive_path, archive_content).with_context(|| {
                    format!("Failed to write archive: {}", archive_path.display())
                })?;
            }
        }
    }

    // Remove whole tasks (in reverse order) - include auto-completed tasks
    let mut whole_task_indices: Vec<_> = whole_tasks_to_remove.into_iter().collect();
    whole_task_indices.extend(tasks_to_auto_complete);
    whole_task_indices.sort_by(|a, b| b.cmp(a));
    whole_task_indices.dedup(); // Remove duplicates in case a task was both manually selected and auto-completed

    for task_idx in whole_task_indices {
        todo_file.tasks.remove(task_idx);
    }

    // Save updated todo file
    let new_content = generate_todo_file(&todo_file);
    fs::write(todo_path, new_content)
        .with_context(|| format!("Failed to write file: {}", todo_path.display()))?;

    let total_processed = tasks_to_archive.len() + auto_completed_tasks.len();
    let action_verb = if should_archive {
        "Archived"
    } else {
        "Removed"
    };
    println!(
        "{}✓ {} {} item(s){}",
        color::Fg(color::Green),
        action_verb,
        total_processed,
        color::Fg(color::Reset)
    );

    for task in tasks_to_archive {
        println!(
            "  {}{}{}",
            color::Fg(color::Red),
            task.text,
            color::Fg(color::Reset)
        );
    }

    // Show auto-completed tasks
    if !auto_completed_tasks.is_empty() {
        for task in auto_completed_tasks {
            println!(
                "  {}{} (auto-completed - all subtasks done){}",
                color::Fg(color::Magenta),
                task.text,
                color::Fg(color::Reset)
            );
        }
    }

    Ok(())
}

/// Archive specified tasks or subtasks
pub fn archive_items(todo_path: &Path, archive_path: &Path, refs: &[String]) -> Result<()> {
    process_items_for_removal(todo_path, refs, Some(archive_path))
}

/// Remove items without archiving
pub fn remove_items(todo_path: &Path, refs: &[String]) -> Result<()> {
    process_items_for_removal(todo_path, refs, None)
}

/// Opens the todo file in the user's preferred editor
pub fn edit_note(todo_path: &Path) -> Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());

    // Create the file if it doesn't exist
    if !todo_path.exists() {
        let empty_file = TodoFile::new("TODOs".to_string());
        let content = generate_todo_file(&empty_file);
        fs::write(todo_path, content)
            .with_context(|| format!("Failed to write file: {}", todo_path.display()))?;
    }

    let status = Command::new(&editor)
        .arg(todo_path)
        .status()
        .with_context(|| format!("Failed to run editor: {}", editor))?;

    if !status.success() {
        println!(
            "{}Editor exited with error code: {}{}",
            color::Fg(color::Red),
            status.code().unwrap_or(1),
            color::Fg(color::Reset)
        );
    }

    Ok(())
}
