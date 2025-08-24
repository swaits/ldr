//! Pure functions for content manipulation (legacy compatibility).
//!
//! This module maintains backward compatibility with existing tests while
//! the main functionality has moved to the markdown module for the new format.

use std::collections::HashSet;

/// Pure function that prepends new text to existing content (legacy format).
/// This is maintained for backward compatibility with existing tests.
#[allow(dead_code)]
pub fn add_entry_to_content(existing_content: &str, text: &str) -> String {
    if existing_content.trim().is_empty() {
        format!("{}\n", text)
    } else {
        format!("{}\n{}", text, existing_content.trim_end())
    }
}

/// Pure function that moves specified lines to the top of the content (legacy format).
/// This is maintained for backward compatibility with existing tests.
#[allow(dead_code)]
pub fn prioritize_items_in_content(
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

/// Pure function that removes specified lines from content (legacy format).
/// This is maintained for backward compatibility with existing tests.
#[allow(dead_code)]
pub fn archive_items_in_content(
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
