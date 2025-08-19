//! Migration utilities for converting plain text files to Markdown format.
//!
//! This module handles the one-time conversion from the old plain text format
//! to the new Markdown format with Default list structure.

use crate::markdown::{Task, TodoFile, ArchiveFile};
use std::fs;
use std::path::Path;

/// Check if migration is needed (plain text files exist but Markdown files don't)
pub fn needs_migration(note_path: &Path, archive_path: &Path, todo_md_path: &Path, archive_md_path: &Path) -> bool {
    let has_old_files = note_path.exists() || archive_path.exists();
    let has_new_files = todo_md_path.exists() || archive_md_path.exists();
    
    has_old_files && !has_new_files
}

/// Migrate plain text note.txt to Markdown todos.md
pub fn migrate_note_file(note_path: &Path) -> Result<TodoFile, String> {
    if !note_path.exists() {
        return Ok(create_empty_todo_file());
    }

    let content = fs::read_to_string(note_path)
        .map_err(|e| format!("Failed to read note file: {}", e))?;

    let mut todo_file = TodoFile::new("TODOs".to_string());

    // Split content into lines and create tasks
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let task = Task::new(trimmed.to_string());
            todo_file.add_task(task);
        }
    }

    Ok(todo_file)
}

/// Migrate plain text archive.txt to Markdown archive.md
pub fn migrate_archive_file(archive_path: &Path) -> Result<ArchiveFile, String> {
    if !archive_path.exists() {
        return Ok(ArchiveFile::new());
    }

    let content = fs::read_to_string(archive_path)
        .map_err(|e| format!("Failed to read archive file: {}", e))?;

    let mut archive_file = ArchiveFile::new();
    let mut tasks = Vec::new();

    // Split content into lines and create tasks for today's date
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let task = Task::new(trimmed.to_string());
            tasks.push(task);
        }
    }

    if !tasks.is_empty() {
        archive_file.add_items_for_today("Default", tasks);
    }

    Ok(archive_file)
}

/// Create an empty todo file
pub fn create_empty_todo_file() -> TodoFile {
    TodoFile::new("TODOs".to_string())
}

/// Create backup files before migration
pub fn create_backups(note_path: &Path, archive_path: &Path) -> Result<(), String> {
    if note_path.exists() {
        let backup_path = note_path.with_extension("txt.bak");
        fs::copy(note_path, &backup_path)
            .map_err(|e| format!("Failed to backup note file to {:?}: {}", backup_path, e))?;
        println!("Created backup: {:?}", backup_path);
    }

    if archive_path.exists() {
        let backup_path = archive_path.with_extension("txt.bak");
        fs::copy(archive_path, &backup_path)
            .map_err(|e| format!("Failed to backup archive file to {:?}: {}", backup_path, e))?;
        println!("Created backup: {:?}", backup_path);
    }

    Ok(())
}

/// Perform the complete migration process
pub fn perform_migration(
    note_path: &Path,
    archive_path: &Path,
    todo_md_path: &Path,
    archive_md_path: &Path,
) -> Result<(), String> {
    println!("🔄 Migrating from plain text to Markdown format...");

    // Create backups first
    create_backups(note_path, archive_path)?;

    // Migrate note file
    let todo_file = migrate_note_file(note_path)?;
    let todo_content = crate::markdown::generate_todo_file(&todo_file);
    
    fs::write(todo_md_path, todo_content)
        .map_err(|e| format!("Failed to write todos.md: {}", e))?;

    // Migrate archive file  
    let archive_file = migrate_archive_file(archive_path)?;
    let archive_content = crate::markdown::generate_archive_file(&archive_file);
    
    fs::write(archive_md_path, archive_content)
        .map_err(|e| format!("Failed to write archive.md: {}", e))?;

    // Count migrated items
    let todo_count = todo_file.task_count();
    let archive_count = archive_file.entries.first()
        .and_then(|e| e.lists.get("Default"))
        .map(|tasks| tasks.len())
        .unwrap_or(0);

    println!("✅ Migration completed successfully!");
    println!("   • Migrated {} todo items", todo_count);
    println!("   • Migrated {} archive items", archive_count);
    println!("   • Created todos.md and archive.md");
    println!("   • Original files backed up with .bak extension");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file_with_content(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_migrate_note_file() {
        let content = "First task\nSecond task\nThird task\n";
        let file = create_test_file_with_content(content);
        
        let result = migrate_note_file(file.path()).unwrap();
        
        assert_eq!(result.title, "TODOs");
        assert_eq!(result.tasks.len(), 3);
        assert_eq!(result.tasks[0].text, "First task");
        assert_eq!(result.tasks[1].text, "Second task");
        assert_eq!(result.tasks[2].text, "Third task");
    }

    #[test]
    fn test_migrate_empty_note_file() {
        let file = create_test_file_with_content("");
        let result = migrate_note_file(file.path()).unwrap();
        
        assert_eq!(result.title, "TODOs");
        assert_eq!(result.tasks.len(), 0);
    }

    #[test]
    fn test_migrate_archive_file() {
        let content = "Completed task 1\nCompleted task 2\n";
        let file = create_test_file_with_content(content);
        
        let result = migrate_archive_file(file.path()).unwrap();
        
        assert_eq!(result.title, "Archive");
        assert_eq!(result.entries.len(), 1);
        
        let entry = &result.entries[0];
        let tasks = entry.lists.get("Default").unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].text, "Completed task 1");
        assert_eq!(tasks[1].text, "Completed task 2");
    }

    #[test]
    fn test_migrate_nonexistent_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent.txt");
        
        let todo_result = migrate_note_file(&nonexistent).unwrap();
        assert!(todo_result.is_empty());
        
        let archive_result = migrate_archive_file(&nonexistent).unwrap();
        assert!(archive_result.entries.is_empty());
    }

    #[test]
    fn test_needs_migration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let note_path = temp_dir.path().join("note.txt");
        let archive_path = temp_dir.path().join("archive.txt");
        let todo_md_path = temp_dir.path().join("todos.md");
        let archive_md_path = temp_dir.path().join("archive.md");

        // No files exist - no migration needed
        assert!(!needs_migration(&note_path, &archive_path, &todo_md_path, &archive_md_path));

        // Create old file
        fs::write(&note_path, "test").unwrap();
        
        // Old file exists, no new files - migration needed
        assert!(needs_migration(&note_path, &archive_path, &todo_md_path, &archive_md_path));

        // Create new file too
        fs::write(&todo_md_path, "test").unwrap();
        
        // Both old and new files exist - no migration needed
        assert!(!needs_migration(&note_path, &archive_path, &todo_md_path, &archive_md_path));
    }
}