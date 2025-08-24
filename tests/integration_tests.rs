//! Integration tests for the LDR CLI application
//!
//! These tests verify the entire application works correctly by running actual CLI commands
//! with temporary data directories. Tests cover all functionality and edge cases according to POLS.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Test fixture that provides isolated temporary directories for each test
struct TestEnv {
    _temp_dir: TempDir,
    data_dir: PathBuf,
    binary_path: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let data_dir = temp_dir.path().join("ldr");
        fs::create_dir_all(&data_dir).expect("Failed to create ldr data directory");

        // Build the binary if it doesn't exist or is out of date
        let binary_path = Self::ensure_binary_built();

        Self {
            _temp_dir: temp_dir,
            data_dir,
            binary_path,
        }
    }

    fn ensure_binary_built() -> PathBuf {
        let output = Command::new("cargo")
            .args(&["build", "--bin", "ldr"])
            .output()
            .expect("Failed to build ldr binary");

        if !output.status.success() {
            panic!(
                "Failed to build binary: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let mut binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        binary_path.push("target/debug/ldr");
        binary_path
    }

    /// Run ldr command with arguments in the isolated environment
    fn run_ldr(&self, args: &[&str]) -> CommandResult {
        let output = Command::new(&self.binary_path)
            .args(args)
            .env("XDG_DATA_HOME", &self.data_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute ldr command");

        CommandResult {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// Get the path to the todos.md file
    fn todos_path(&self) -> PathBuf {
        self.data_dir.join("ldr/todos.md")
    }

    /// Get the path to the archive.md file
    fn archive_path(&self) -> PathBuf {
        self.data_dir.join("ldr/archive.md")
    }

    /// Read the contents of todos.md
    fn read_todos(&self) -> String {
        fs::read_to_string(self.todos_path()).unwrap_or_default()
    }

    /// Read the contents of archive.md
    fn read_archive(&self) -> String {
        fs::read_to_string(self.archive_path()).unwrap_or_default()
    }
}

#[derive(Debug)]
struct CommandResult {
    status: i32,
    stdout: String,
    stderr: String,
}

impl CommandResult {
    fn assert_success(&self) {
        if self.status != 0 {
            panic!(
                "Command failed with status {}: stderr: {}",
                self.status, self.stderr
            );
        }
    }

    fn assert_failure(&self) {
        // For LDR, failure is indicated by error messages in stdout, not exit code
        if !self.stdout.contains("Invalid")
            && !self.stderr.contains("Error")
            && !self.stdout.is_empty()
        {
            panic!("Command unexpectedly succeeded: stdout: {}", self.stdout);
        }
    }
}

#[cfg(test)]
mod basic_operations {
    use super::*;

    #[test]
    fn test_add_and_list_basic() {
        let env = TestEnv::new();

        // Add first item
        let result = env.run_ldr(&["add", "First task"]);
        result.assert_success();
        assert!(result.stdout.contains("✓ Added: First task"));

        // Add second item (should go to top)
        let result = env.run_ldr(&["add", "Second task"]);
        result.assert_success();

        // List items
        let result = env.run_ldr(&["ls"]);
        result.assert_success();

        // Check that second task appears first (POLS: newest items at top)
        // Note: Output includes ANSI color codes, so we check for task text
        assert!(result.stdout.contains("1. Second task"));
        assert!(result.stdout.contains("2. First task"));
    }

    #[test]
    fn test_empty_list() {
        let env = TestEnv::new();

        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("No notes yet."));
    }

    #[test]
    fn test_list_with_limits() {
        let env = TestEnv::new();

        // Add 10 items
        for i in 1..=10 {
            let result = env.run_ldr(&["add", &format!("Task {}", i)]);
            result.assert_success();
        }

        // Test default limit (5)
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        let line_count = result.stdout.lines().count();
        // Should show 5 items plus "... and X more items" line
        assert!(line_count >= 5);
        assert!(result.stdout.contains("and 5 more items"));

        // Test custom limit
        let result = env.run_ldr(&["ls", "-n", "3"]);
        result.assert_success();
        let lines: Vec<&str> = result.stdout.lines().collect();
        assert!(lines.len() >= 3);

        // Test show all
        let result = env.run_ldr(&["ls", "--all"]);
        result.assert_success();
        assert!(!result.stdout.contains("more items"));
    }
}

#[cfg(test)]
mod subtask_operations {
    use super::*;

    #[test]
    fn test_add_subtask() {
        let env = TestEnv::new();

        // Add main task
        let result = env.run_ldr(&["add", "Main task"]);
        result.assert_success();

        // Add subtask
        let result = env.run_ldr(&["add", "Subtask A", "--under", "1"]);
        result.assert_success();
        assert!(result
            .stdout
            .contains("✓ Added subtask to task 1: Subtask A"));

        // Add another subtask
        let result = env.run_ldr(&["add", "Subtask B", "--under", "1"]);
        result.assert_success();

        // List and verify structure
        let result = env.run_ldr(&["ls"]);
        result.assert_success();

        // Verify main task and subtasks appear in order added
        assert!(result.stdout.contains("1. Main task"));
        assert!(result.stdout.contains("a. Subtask A")); // First subtask
        assert!(result.stdout.contains("b. Subtask B")); // Second subtask
    }

    #[test]
    fn test_add_subtask_invalid_parent() {
        let env = TestEnv::new();

        // Try to add subtask to non-existent task
        let result = env.run_ldr(&["add", "Orphan subtask", "--under", "1"]);
        result.assert_failure();
        assert!(result.stderr.contains("Invalid task number: 1"));
    }

    #[test]
    fn test_subtask_numbering() {
        let env = TestEnv::new();

        // Add main task
        env.run_ldr(&["add", "Task with many subtasks"]);

        // Add multiple subtasks
        for i in 1..=5 {
            let result = env.run_ldr(&["add", &format!("Subtask {}", i), "--under", "1"]);
            result.assert_success();
        }

        let result = env.run_ldr(&["ls", "--all"]);
        result.assert_success();

        // Verify letter sequence (in order added: a, b, c, d, e)
        assert!(result.stdout.contains("a. Subtask 1"));
        assert!(result.stdout.contains("b. Subtask 2"));
        assert!(result.stdout.contains("c. Subtask 3"));
        assert!(result.stdout.contains("d. Subtask 4"));
        assert!(result.stdout.contains("e. Subtask 5"));
    }
}

#[cfg(test)]
mod prioritization_tests {
    use super::*;

    #[test]
    fn test_prioritize_basic() {
        let env = TestEnv::new();

        // Add tasks
        env.run_ldr(&["add", "Task A"]);
        env.run_ldr(&["add", "Task B"]);
        env.run_ldr(&["add", "Task C"]);

        // Prioritize task 3 (Task A)
        let result = env.run_ldr(&["up", "3"]);
        result.assert_success();
        assert!(result.stdout.contains("✓ Prioritized 1 task(s)"));

        // Verify new order
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        let lines: Vec<&str> = result.stdout.lines().collect();
        assert!(lines[0].contains("1. Task A")); // Moved to top
        assert!(lines[1].contains("2. Task C"));
        assert!(lines[2].contains("3. Task B"));
    }

    #[test]
    fn test_prioritize_multiple() {
        let env = TestEnv::new();

        // Add tasks
        env.run_ldr(&["add", "Task A"]);
        env.run_ldr(&["add", "Task B"]);
        env.run_ldr(&["add", "Task C"]);
        env.run_ldr(&["add", "Task D"]);

        // Prioritize tasks 4, 2 (in that order)
        let result = env.run_ldr(&["up", "4", "2"]);
        result.assert_success();

        // Verify command-line order preserved (POLS)
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        let lines: Vec<&str> = result.stdout.lines().collect();
        assert!(lines[0].contains("1. Task A")); // 4th item moved to top
        assert!(lines[1].contains("2. Task C")); // 2nd item moved to second
        assert!(lines[2].contains("3. Task D")); // Remaining items
        assert!(lines[3].contains("4. Task B"));
    }

    #[test]
    fn test_prioritize_subtask_moves_parent() {
        let env = TestEnv::new();

        // Add tasks with subtasks
        env.run_ldr(&["add", "Task A"]);
        env.run_ldr(&["add", "Subtask A1", "--under", "1"]);
        env.run_ldr(&["add", "Task B"]);
        env.run_ldr(&["add", "Subtask B1", "--under", "2"]);

        // Prioritize subtask - should move entire parent task
        let result = env.run_ldr(&["up", "2a"]); // Task A is now at position 2
        result.assert_success();

        // Verify parent task moved (POLS: subtask reference affects parent)
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("1. Task A")); // Parent moved to top
        assert!(result.stdout.contains("a. Subtask A1"));
    }

    #[test]
    fn test_prioritize_invalid_reference() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Task A"]);

        let result = env.run_ldr(&["up", "5"]);
        result.assert_failure();

        let result = env.run_ldr(&["up", "1z"]); // Invalid subtask
        result.assert_failure();
    }
}

#[cfg(test)]
mod archiving_tests {
    use super::*;

    #[test]
    fn test_archive_single_task() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Task to complete"]);

        let result = env.run_ldr(&["do", "1"]);
        result.assert_success();
        assert!(result.stdout.contains("✓ Archived 1 item(s)"));

        // Verify task removed from todos
        let todos = env.read_todos();
        assert!(!todos.contains("Task to complete"));

        // Verify task in archive
        let archive = env.read_archive();
        assert!(archive.contains("Task to complete"));
        assert!(archive.contains(&chrono::Local::now().format("%Y-%m-%d").to_string()));
    }

    #[test]
    fn test_archive_subtask() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Main task"]);
        env.run_ldr(&["add", "Subtask A", "--under", "1"]);
        env.run_ldr(&["add", "Subtask B", "--under", "1"]);

        // Archive one subtask
        let result = env.run_ldr(&["do", "1a"]);
        result.assert_success();

        // Main task should still exist with remaining subtask
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("Main task"));
        assert!(result.stdout.contains("Subtask B"));
        assert!(!result.stdout.contains("Subtask A"));

        // Archive should contain the subtask
        let archive = env.read_archive();
        assert!(archive.contains("Subtask A"));
    }

    #[test]
    fn test_auto_complete_parent_when_all_subtasks_done() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Main task"]);
        env.run_ldr(&["add", "Subtask A", "--under", "1"]);
        env.run_ldr(&["add", "Subtask B", "--under", "1"]);

        // Archive all subtasks
        let result = env.run_ldr(&["do", "1a", "1b"]);
        result.assert_success();

        // Main task should be auto-completed (POLS: completing all subtasks completes parent)
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(!result.stdout.contains("Main task"));

        // Archive should contain subtasks AND parent task
        let archive = env.read_archive();
        assert!(archive.contains("Subtask A"));
        assert!(archive.contains("Subtask B"));
        assert!(archive.contains("Main task"));
    }

    #[test]
    fn test_archive_whole_task_with_subtasks() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Complete project"]);
        env.run_ldr(&["add", "Write code", "--under", "1"]);
        env.run_ldr(&["add", "Write tests", "--under", "1"]);

        // Archive entire task (should include all subtasks)
        let result = env.run_ldr(&["do", "1"]);
        result.assert_success();

        // Nothing should remain in todos
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("No notes yet"));

        // Archive should contain complete task structure
        let archive = env.read_archive();
        assert!(archive.contains("Complete project"));
        assert!(archive.contains("Write code"));
        assert!(archive.contains("Write tests"));
    }
}

#[cfg(test)]
mod removal_tests {
    use super::*;

    #[test]
    fn test_remove_vs_archive() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Task to remove"]);
        env.run_ldr(&["add", "Task to archive"]);

        // Remove one task
        let result = env.run_ldr(&["rm", "2"]);
        result.assert_success();

        // Archive another task
        let result = env.run_ldr(&["do", "1"]);
        result.assert_success();

        // Only archived task should be in archive
        let archive = env.read_archive();
        assert!(archive.contains("Task to archive"));
        assert!(!archive.contains("Task to remove"));

        // Both should be gone from todos
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("No notes yet"));
    }

    #[test]
    fn test_remove_subtask() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Main task"]);
        env.run_ldr(&["add", "Keep this", "--under", "1"]);
        env.run_ldr(&["add", "Remove this", "--under", "1"]);

        // Remove second subtask (1b)
        let result = env.run_ldr(&["rm", "1b"]);
        result.assert_success();

        // Main task should remain with first subtask only
        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("Main task"));
        assert!(result.stdout.contains("Keep this"));
        assert!(!result.stdout.contains("Remove this"));

        // Archive should be empty
        let archive = env.read_archive();
        assert!(!archive.contains("Remove this"));
    }
}

#[cfg(test)]
mod filtering_tests {
    use super::*;

    #[test]
    fn test_filtering_basic() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "read: Book about Rust"]);
        env.run_ldr(&["add", "write: Article about Go"]);
        env.run_ldr(&["add", "read: Documentation"]);
        env.run_ldr(&["add", "@work: Review PR"]);

        // Filter by "read:"
        let result = env.run_ldr(&["ls", "read:"]);
        result.assert_success();
        assert!(result.stdout.contains("read: Book about Rust"));
        assert!(result.stdout.contains("read: Documentation"));
        assert!(!result.stdout.contains("write: Article"));
        assert!(!result.stdout.contains("@work: Review"));

        // Filter by "@work"
        let result = env.run_ldr(&["ls", "@work"]);
        result.assert_success();
        assert!(result.stdout.contains("@work: Review PR"));
        assert!(!result.stdout.contains("read:"));
    }

    #[test]
    fn test_filtering_case_insensitive() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "READ: Book"]);
        env.run_ldr(&["add", "read: Article"]);
        env.run_ldr(&["add", "ReAd: Mixed"]);

        let result = env.run_ldr(&["ls", "read:"]);
        result.assert_success();
        assert!(result.stdout.contains("READ: Book"));
        assert!(result.stdout.contains("read: Article"));
        assert!(result.stdout.contains("ReAd: Mixed"));
    }

    #[test]
    fn test_filtering_with_limits() {
        let env = TestEnv::new();

        // Add many matching items
        for i in 1..=10 {
            env.run_ldr(&["add", &format!("read: Book {}", i)]);
        }
        env.run_ldr(&["add", "write: Article"]);

        // Test filtering respects limits
        let result = env.run_ldr(&["ls", "-n", "3", "read:"]);
        result.assert_success();
        let matching_lines: Vec<&str> = result
            .stdout
            .lines()
            .filter(|line| line.contains("read:"))
            .collect();
        assert!(matching_lines.len() <= 3);

        // Test filtering with --all
        let result = env.run_ldr(&["ls", "--all", "read:"]);
        result.assert_success();
        let matching_lines: Vec<&str> = result
            .stdout
            .lines()
            .filter(|line| line.contains("read:"))
            .collect();
        assert_eq!(matching_lines.len(), 10);
    }

    #[test]
    fn test_filtering_no_matches() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Task A"]);
        env.run_ldr(&["add", "Task B"]);

        let result = env.run_ldr(&["ls", "nonexistent"]);
        result.assert_success();
        assert!(result
            .stdout
            .contains("No items found matching filter: \"nonexistent\""));
    }
}

#[cfg(test)]
mod error_handling {
    use super::*;

    #[test]
    fn test_invalid_task_references() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Task A"]);

        // Test various invalid references
        let invalid_refs = vec!["0", "99", "1z", "a1", "1ab", "1-2", ""];

        for invalid_ref in invalid_refs {
            if !invalid_ref.is_empty() {
                let result = env.run_ldr(&["up", invalid_ref]);
                result.assert_failure();
            }
        }
    }

    #[test]
    fn test_operations_on_empty_file() {
        let env = TestEnv::new();

        // Try operations on empty file
        let result = env.run_ldr(&["up", "1"]);
        result.assert_success(); // Should handle gracefully
        assert!(result.stdout.contains("No notes found"));

        let result = env.run_ldr(&["do", "1"]);
        result.assert_success();
        assert!(result.stdout.contains("No notes found"));

        let result = env.run_ldr(&["rm", "1"]);
        result.assert_success();
    }

    #[test]
    fn test_subtask_references_without_parent() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Task without subtasks"]);

        // Try to reference non-existent subtask
        let result = env.run_ldr(&["up", "1a"]);
        result.assert_failure();

        let result = env.run_ldr(&["do", "1a"]);
        result.assert_failure();

        let result = env.run_ldr(&["rm", "1a"]);
        result.assert_failure();
    }
}

#[cfg(test)]
mod edit_functionality {
    use super::*;

    #[test]
    fn test_edit_creates_file_if_not_exists() {
        let env = TestEnv::new();

        // Set EDITOR to a command that just touches the file and exits
        let result = Command::new(&env.binary_path)
            .args(&["edit"])
            .env("XDG_DATA_HOME", &env.data_dir)
            .env("EDITOR", "touch") // Will just touch the file
            .output()
            .expect("Failed to execute edit command");

        // Should succeed (touch command succeeds)
        assert_eq!(result.status.code().unwrap_or(-1), 0);

        // File should exist with proper structure
        let todos = env.read_todos();
        assert!(todos.contains("# TODOs"));
    }

    #[test]
    fn test_edit_aliases() {
        let env = TestEnv::new();

        // Test all aliases work by using echo to verify they're called
        let aliases = vec!["edit", "e", "scan", "s", "review", "r"];

        for alias in aliases {
            let result = Command::new(&env.binary_path)
                .args(&[alias])
                .env("XDG_DATA_HOME", &env.data_dir)
                .env("EDITOR", "/bin/echo")
                .output()
                .expect(&format!("Failed to execute {} command", alias));

            // Should succeed and echo the file path
            assert_eq!(result.status.code().unwrap_or(-1), 0);
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                stdout.contains("todos.md"),
                "Alias '{}' didn't work: {}",
                alias,
                stdout
            );
        }
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn test_migration_from_plain_text() {
        let env = TestEnv::new();

        // Create old-style plain text files
        let old_note_path = env.data_dir.join("ldr/note.txt");
        let old_archive_path = env.data_dir.join("ldr/archive.txt");

        fs::create_dir_all(old_note_path.parent().unwrap()).unwrap();

        // Write old-style content
        fs::write(&old_note_path, "Task A\nTask B with details\nTask C\n").unwrap();
        fs::write(&old_archive_path, "Completed task 1\nCompleted task 2\n").unwrap();

        // Run any command to trigger migration
        let result = env.run_ldr(&["ls"]);
        result.assert_success();

        // Check that markdown files were created
        assert!(env.todos_path().exists());
        assert!(env.archive_path().exists());

        // Verify content was migrated correctly
        let todos = env.read_todos();
        assert!(todos.contains("# TODOs"));
        assert!(todos.contains("- Task A"));
        assert!(todos.contains("- Task B with details"));
        assert!(todos.contains("- Task C"));

        let archive = env.read_archive();
        assert!(archive.contains("# Archive"));
        assert!(archive.contains("- Completed task 1"));
        assert!(archive.contains("- Completed task 2"));

        // Verify old files were backed up (not deleted)
        assert!(old_note_path.with_extension("txt.bak").exists());
        assert!(old_archive_path.with_extension("txt.bak").exists());
    }

    #[test]
    fn test_no_migration_when_markdown_exists() {
        let env = TestEnv::new();

        // Create markdown file first
        fs::create_dir_all(env.todos_path().parent().unwrap()).unwrap();
        fs::write(&env.todos_path(), "# TODOs\n\n- Existing task\n").unwrap();

        // Create old-style file
        let old_note_path = env.data_dir.join("ldr/note.txt");
        fs::write(&old_note_path, "Old task\n").unwrap();

        // Run command
        let result = env.run_ldr(&["ls"]);
        result.assert_success();

        // Verify markdown file unchanged (no migration happened)
        let todos = env.read_todos();
        assert!(todos.contains("Existing task"));
        assert!(!todos.contains("Old task"));
    }
}

#[cfg(test)]
mod pols_compliance {
    use super::*;

    #[test]
    fn test_newest_items_at_top() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Old task"]);
        std::thread::sleep(std::time::Duration::from_millis(10));
        env.run_ldr(&["add", "New task"]);

        let result = env.run_ldr(&["ls"]);
        result.assert_success();

        assert!(result.stdout.contains("1. New task"));
        assert!(result.stdout.contains("2. Old task"));
    }

    #[test]
    fn test_prioritization_order_predictable() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "A"]);
        env.run_ldr(&["add", "B"]);
        env.run_ldr(&["add", "C"]);
        env.run_ldr(&["add", "D"]);

        // Test prioritization is predictable
        env.run_ldr(&["up", "4", "2"]); // Move A (4th) and C (2nd) to top

        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("1. A"));
        assert!(result.stdout.contains("2. C"));
        assert!(result.stdout.contains("3. D"));
        assert!(result.stdout.contains("4. B"));
    }

    #[test]
    fn test_subtask_operations_affect_parent_logically() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Project A"]);
        env.run_ldr(&["add", "Task 1", "--under", "1"]);
        env.run_ldr(&["add", "Project B"]);

        // Prioritizing subtask should move parent project
        env.run_ldr(&["up", "2a"]); // Project A is now at position 2

        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        // Project A should be at top (moved because of subtask reference)
        assert!(result.stdout.contains("1. Project A"));
        assert!(result.stdout.contains("a. Task 1"));
    }

    #[test]
    fn test_completion_cascades_logically() {
        let env = TestEnv::new();

        env.run_ldr(&["add", "Big project"]);
        env.run_ldr(&["add", "Only subtask", "--under", "1"]);

        // Completing the only subtask should complete parent
        env.run_ldr(&["do", "1a"]);

        let result = env.run_ldr(&["ls"]);
        result.assert_success();
        assert!(result.stdout.contains("No notes yet"));

        // Both should be in archive
        let archive = env.read_archive();
        assert!(archive.contains("Only subtask"));
        assert!(archive.contains("Big project"));
    }
}
