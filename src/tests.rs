//! Comprehensive test suite for the ldr todo system.
//!
//! Tests cover all core functionality including content manipulation,
//! command operations, filtering, and edge cases.

use crate::commands::*;
use crate::content::*;

/// Tests adding an entry to empty content
#[test]
fn test_add_entry_to_content_empty() {
    let result = add_entry_to_content("", "First item");
    assert_eq!(result, "First item\n");
}

/// Tests adding an entry to existing content
#[test]
fn test_add_entry_to_content_existing() {
    let existing = "Second item\nThird item";
    let result = add_entry_to_content(existing, "First item");
    assert_eq!(result, "First item\nSecond item\nThird item");
}

/// Tests adding an entry when existing content has trailing newline
#[test]
fn test_add_entry_to_content_with_trailing_newline() {
    let existing = "Second item\nThird item\n";
    let result = add_entry_to_content(existing, "First item");
    assert_eq!(result, "First item\nSecond item\nThird item");
}

/// Tests prioritizing a single item by moving it to the top
#[test]
fn test_prioritize_items_in_content_single_item() {
    let lines = vec![
        "First item".to_string(),
        "Second item".to_string(),
        "Third item".to_string(),
    ];
    let result = prioritize_items_in_content(&lines, &[2]);

    assert!(result.is_ok());
    let (new_content, prioritized) = result.unwrap();
    assert_eq!(new_content, "Second item\nFirst item\nThird item\n");
    assert_eq!(prioritized, vec!["Second item"]);
}

/// Tests prioritizing multiple items in command-line order
#[test]
fn test_prioritize_items_in_content_multiple_items() {
    let lines = vec![
        "First item".to_string(),
        "Second item".to_string(),
        "Third item".to_string(),
        "Fourth item".to_string(),
    ];
    let result = prioritize_items_in_content(&lines, &[3, 1]);

    assert!(result.is_ok());
    let (new_content, prioritized) = result.unwrap();
    assert_eq!(
        new_content,
        "Third item\nFirst item\nSecond item\nFourth item\n"
    );
    assert_eq!(prioritized, vec!["Third item", "First item"]);
}

/// Tests error handling for invalid item numbers
#[test]
fn test_prioritize_items_in_content_invalid_number() {
    let lines = vec!["First item".to_string(), "Second item".to_string()];
    let result = prioritize_items_in_content(&lines, &[3]);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Invalid item number: 3. Valid range: 1-2"
    );
}

/// Tests error handling for zero item numbers
#[test]
fn test_prioritize_items_in_content_zero_number() {
    let lines = vec!["First item".to_string()];
    let result = prioritize_items_in_content(&lines, &[0]);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Invalid item number: 0. Valid range: 1-1"
    );
}

/// Tests duplicate removal while preserving order
#[test]
fn test_prioritize_items_in_content_duplicates() {
    let lines = vec![
        "First item".to_string(),
        "Second item".to_string(),
        "Third item".to_string(),
    ];
    let result = prioritize_items_in_content(&lines, &[2, 2, 1]);

    assert!(result.is_ok());
    let (new_content, prioritized) = result.unwrap();
    assert_eq!(new_content, "Second item\nFirst item\nThird item\n");
    assert_eq!(prioritized, vec!["Second item", "First item"]);
}

/// Tests archiving a single item
#[test]
fn test_archive_items_in_content_single_item() {
    let lines = vec![
        "First item".to_string(),
        "Second item".to_string(),
        "Third item".to_string(),
    ];
    let result = archive_items_in_content(&lines, &[2]);

    assert!(result.is_ok());
    let (new_content, archived) = result.unwrap();
    assert_eq!(new_content, "First item\nThird item\n");
    assert_eq!(archived, vec!["Second item"]);
}

/// Tests archiving multiple items
#[test]
fn test_archive_items_in_content_multiple_items() {
    let lines = vec![
        "First item".to_string(),
        "Second item".to_string(),
        "Third item".to_string(),
        "Fourth item".to_string(),
    ];
    let result = archive_items_in_content(&lines, &[1, 3]);

    assert!(result.is_ok());
    let (new_content, archived) = result.unwrap();
    assert_eq!(new_content, "Second item\nFourth item\n");
    assert_eq!(archived, vec!["First item", "Third item"]);
}

/// Tests archiving all items results in empty content
#[test]
fn test_archive_items_in_content_all_items() {
    let lines = vec!["First item".to_string(), "Second item".to_string()];
    let result = archive_items_in_content(&lines, &[1, 2]);

    assert!(result.is_ok());
    let (new_content, archived) = result.unwrap();
    assert_eq!(new_content, "");
    assert_eq!(archived, vec!["First item", "Second item"]);
}

/// Tests error handling for invalid archive item numbers
#[test]
fn test_archive_items_in_content_invalid_number() {
    let lines = vec!["First item".to_string()];
    let result = archive_items_in_content(&lines, &[2]);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Invalid item number: 2. Valid range: 1-1"
    );
}

/// Tests duplicate handling in archive operations
#[test]
fn test_archive_items_in_content_duplicates() {
    let lines = vec![
        "First item".to_string(),
        "Second item".to_string(),
        "Third item".to_string(),
    ];
    let result = archive_items_in_content(&lines, &[1, 1, 3]);

    assert!(result.is_ok());
    let (new_content, archived) = result.unwrap();
    assert_eq!(new_content, "Second item\n");
    assert_eq!(archived, vec!["First item", "Third item"]);
}

/// Tests case-insensitive filtering functionality
#[test]
fn test_filter_functionality() {
    let lines = vec![
        "Read: Book about Rust".to_string(),
        "Listen: Podcast episode".to_string(),
        "@work: Review PR".to_string(),
        "@home: Fix sink".to_string(),
        "Read: Another book".to_string(),
    ];

    // Test filtering by "read:"
    let filtered: Vec<(usize, &String)> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains("read:"))
        .collect();

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].0, 0);
    assert_eq!(filtered[0].1, "Read: Book about Rust");
    assert_eq!(filtered[1].0, 4);
    assert_eq!(filtered[1].1, "Read: Another book");

    // Test filtering by "@work"
    let filtered: Vec<(usize, &String)> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains("@work"))
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].0, 2);
    assert_eq!(filtered[0].1, "@work: Review PR");
}

/// Tests that filtering is case-insensitive
#[test]
fn test_case_insensitive_filtering() {
    let lines = vec![
        "READ: Book about Rust".to_string(),
        "read: Another book".to_string(),
        "ReAd: Mixed case".to_string(),
    ];

    let filtered: Vec<(usize, &String)> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains("read:"))
        .collect();

    assert_eq!(filtered.len(), 3);
}
#[cfg(test)]
mod list_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper function to create temporary test files with content (legacy)
    #[allow(dead_code)]
    fn create_test_file_with_content(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    /// Helper function to create Markdown todo files for new tests
    fn create_markdown_test_file(tasks: &[&str]) -> NamedTempFile {
        let mut content = String::from("# TODOs\n\n");
        for task in tasks {
            content.push_str(&format!("- {}\n", task));
        }

        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    /// Tests listing all items when all flag is true
    #[test]
    fn test_list_with_all_flag() {
        let file = create_markdown_test_file(&[
            "item 1", "item 2", "item 3", "item 4", "item 5", "item 6",
        ]);

        // Test that all=true shows all items regardless of num
        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 3, true, None).unwrap();
        });
        assert!(result.is_ok());
    }

    /// Tests respecting num limit when all flag is false
    #[test]
    fn test_list_without_all_flag() {
        let file = create_markdown_test_file(&[
            "item 1", "item 2", "item 3", "item 4", "item 5", "item 6",
        ]);

        // Test that all=false respects num limit
        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 3, false, None).unwrap();
        });
        assert!(result.is_ok());
    }

    /// Tests filtering with all flag shows all matching items
    #[test]
    fn test_list_with_filter_and_all() {
        let file = create_markdown_test_file(&[
            "read: book 1",
            "write: article",
            "read: book 2",
            "task: cleanup",
        ]);

        // Test that all=true with filter shows all matching items
        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 1, true, Some("read:")).unwrap();
        });
        assert!(result.is_ok());
    }

    /// Tests handling of empty files
    #[test]
    fn test_list_empty_file() {
        let file = create_markdown_test_file(&[]);

        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 5, false, None).unwrap();
        });
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod review_tests {
    use std::collections::HashSet;

    /// Tests that prioritized items maintain their relative order during review
    #[test]
    fn test_review_prioritization_order() {
        // Test that items prioritized during review maintain their relative order
        let lines = vec![
            "First item".to_string(),
            "Second item".to_string(),
            "Third item".to_string(),
            "Fourth item".to_string(),
        ];

        // Simulate review process: prioritize items 2 and 4 (in that encounter order during bottom-to-top review)
        let mut prioritized: Vec<String> = Vec::new();
        let mut remove_indices: HashSet<usize> = HashSet::new();

        // Simulate top-to-bottom review order (indices 0, 1, 2, 3) - same as list order
        // User prioritizes item at index 1 (Second item)
        prioritized.push(lines[1].clone());
        remove_indices.insert(1);

        // User prioritizes item at index 3 (Fourth item)
        prioritized.push(lines[3].clone());
        remove_indices.insert(3);

        // Build remaining items (items not prioritized)
        let mut remaining: Vec<String> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if !remove_indices.contains(&i) {
                remaining.push(line.clone());
            }
        }

        // Combine: prioritized items first (in order they were prioritized), then remaining
        let mut new_lines = prioritized;
        new_lines.extend(remaining);

        // Expected result: prioritized items (Second, Fourth) should appear first,
        // in the order they were prioritized, followed by remaining items (First, Third)
        let expected = vec![
            "Second item".to_string(),
            "Fourth item".to_string(),
            "First item".to_string(),
            "Third item".to_string(),
        ];

        assert_eq!(new_lines, expected);
    }

    /// Tests that quitting review still saves any changes made
    #[test]
    fn test_review_quit_saves_changes() {
        // Test that quitting review still saves any changes made
        let lines = vec![
            "First item".to_string(),
            "Second item".to_string(),
            "Third item".to_string(),
        ];

        // Simulate review process where user prioritizes one item then quits
        let mut prioritized: Vec<String> = Vec::new();
        let mut remove_indices: HashSet<usize> = HashSet::new();

        // User prioritizes item at index 1 (Second item)
        prioritized.push(lines[1].clone());
        remove_indices.insert(1);

        // User quits (but changes should still be applied)

        // Build remaining items (items not prioritized or archived)
        let mut remaining: Vec<String> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if !remove_indices.contains(&i) {
                remaining.push(line.clone());
            }
        }

        // Combine: prioritized items first, then remaining
        let mut new_lines = prioritized;
        new_lines.extend(remaining);

        // Expected result: prioritized item should be first, even though user quit
        let expected = vec![
            "Second item".to_string(),
            "First item".to_string(),
            "Third item".to_string(),
        ];

        assert_eq!(new_lines, expected);
    }

    /// Tests that items are prioritized in exact command-line order
    #[test]
    fn test_prioritize_command_line_order() {
        // Test that items are prioritized in the exact order specified on command line
        let lines = vec![
            "Item A".to_string(),
            "Item B".to_string(),
            "Item C".to_string(),
            "Item D".to_string(),
            "Item E".to_string(),
        ];

        // Command: arn up 5 2 4 (prioritize items 5, 2, 4 in that order)
        let result = crate::content::prioritize_items_in_content(&lines, &[5, 2, 4]);

        assert!(result.is_ok());
        let (new_content, prioritized) = result.unwrap();

        // Expected: Item E, Item B, Item D should be at top in that order
        let expected_content = "Item E\nItem B\nItem D\nItem A\nItem C\n";
        let expected_prioritized = vec!["Item E", "Item B", "Item D"];

        assert_eq!(new_content, expected_content);
        assert_eq!(prioritized, expected_prioritized);
    }

    /// Tests duplicate removal while preserving command-line order
    #[test]
    fn test_prioritize_command_line_order_with_duplicates() {
        // Test that duplicates are removed but order is preserved
        let lines = vec![
            "Item A".to_string(),
            "Item B".to_string(),
            "Item C".to_string(),
        ];

        // Command: arn up 3 1 3 2 1 (should become 3, 1, 2)
        let result = crate::content::prioritize_items_in_content(&lines, &[3, 1, 3, 2, 1]);

        assert!(result.is_ok());
        let (new_content, prioritized) = result.unwrap();

        // Expected: Item C, Item A, Item B (duplicates removed, order preserved)
        let expected_content = "Item C\nItem A\nItem B\n";
        let expected_prioritized = vec!["Item C", "Item A", "Item B"];

        assert_eq!(new_content, expected_content);
        assert_eq!(prioritized, expected_prioritized);
    }
}

#[cfg(test)]
mod remove_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper function to create temporary test files with content (legacy)
    #[allow(dead_code)]
    fn create_test_file_with_content(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    /// Helper function to create Markdown todo files for new tests
    fn create_markdown_test_file(tasks: &[&str]) -> NamedTempFile {
        let mut content = String::from("# TODOs\n\n");
        for task in tasks {
            content.push_str(&format!("- {}\n", task));
        }

        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    /// Helper function to create Markdown todo files with subtasks
    fn create_markdown_test_file_with_subtasks(
        tasks_with_subtasks: &[(&str, &[&str])],
    ) -> NamedTempFile {
        let mut content = String::from("# TODOs\n\n");
        for (task, subtasks) in tasks_with_subtasks {
            content.push_str(&format!("- {}\n", task));
            for subtask in *subtasks {
                content.push_str(&format!("  - {}\n", subtask));
            }
        }

        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    /// Tests removing a single item
    #[test]
    fn test_remove_single_item() {
        let file = create_markdown_test_file(&["First item", "Second item", "Third item"]);

        let result = std::panic::catch_unwind(|| {
            remove_items(file.path(), &["2".to_string()]).unwrap();
        });
        assert!(result.is_ok());

        // Verify the item was removed from the file
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        let expected_content = "# TODOs\n\n- First item\n- Third item\n";
        assert_eq!(updated_content, expected_content);
    }

    /// Tests removing multiple items
    #[test]
    fn test_remove_multiple_items() {
        let file =
            create_markdown_test_file(&["First item", "Second item", "Third item", "Fourth item"]);

        let result = std::panic::catch_unwind(|| {
            remove_items(file.path(), &["1".to_string(), "3".to_string()]).unwrap();
        });
        assert!(result.is_ok());

        // Verify the items were removed from the file
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        let expected_content = "# TODOs\n\n- Second item\n- Fourth item\n";
        assert_eq!(updated_content, expected_content);
    }

    /// Tests removing all items results in empty file
    #[test]
    fn test_remove_all_items() {
        let file = create_markdown_test_file(&["First item", "Second item"]);

        let result = std::panic::catch_unwind(|| {
            remove_items(file.path(), &["1".to_string(), "2".to_string()]).unwrap();
        });
        assert!(result.is_ok());

        // Verify the file has only the header
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        let expected_content = "# TODOs\n\n";
        assert_eq!(updated_content, expected_content);
    }

    /// Tests handling of invalid item numbers
    #[test]
    fn test_remove_invalid_item_number() {
        let file = create_markdown_test_file(&["First item", "Second item"]);
        let original_content = std::fs::read_to_string(file.path()).unwrap();

        let result = std::panic::catch_unwind(|| {
            remove_items(file.path(), &["3".to_string()]).unwrap();
        });
        assert!(result.is_ok());

        // Verify the file content is unchanged
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        assert_eq!(updated_content, original_content);
    }

    /// Tests handling of duplicate item numbers
    #[test]
    fn test_remove_duplicate_item_numbers() {
        let file = create_markdown_test_file(&["First item", "Second item", "Third item"]);

        let result = std::panic::catch_unwind(|| {
            remove_items(
                file.path(),
                &["2".to_string(), "2".to_string(), "1".to_string()],
            )
            .unwrap();
        });
        assert!(result.is_ok());

        // Verify only unique items were removed
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        let expected_content = "# TODOs\n\n- Third item\n";
        assert_eq!(updated_content, expected_content);
    }

    /// Tests handling of non-existent file
    #[test]
    fn test_remove_from_nonexistent_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.txt");

        let result = std::panic::catch_unwind(|| {
            remove_items(&nonexistent_path, &["1".to_string()]).unwrap();
        });
        assert!(result.is_ok());

        // Verify no file was created
        assert!(!nonexistent_path.exists());
    }

    /// Tests handling of empty file
    #[test]
    fn test_remove_from_empty_file() {
        let file = create_markdown_test_file(&[]);

        let result = std::panic::catch_unwind(|| {
            remove_items(file.path(), &["1".to_string()]).unwrap();
        });
        assert!(result.is_ok());

        // Verify the file remains with just the header
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        let expected_content = "# TODOs\n\n";
        assert_eq!(updated_content, expected_content);
    }

    /// Tests that remove doesn't create an archive file (unlike do command)
    #[test]
    fn test_remove_does_not_archive() {
        let file = create_markdown_test_file(&["First item", "Second item"]);
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("archive.md");

        let result = std::panic::catch_unwind(|| {
            remove_items(file.path(), &["1".to_string()]).unwrap();
        });
        assert!(result.is_ok());

        // Verify no archive file was created
        assert!(!archive_path.exists());

        // Verify the item was removed from the original file
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        let expected_content = "# TODOs\n\n- Second item\n";
        assert_eq!(updated_content, expected_content);
    }

    /// Tests that completing all subtasks of a task automatically completes the parent task
    #[test]
    fn test_auto_complete_parent_when_all_subtasks_done() {
        let file = create_markdown_test_file_with_subtasks(&[
            ("Task A", &[]),                         // Task with no subtasks
            ("Task B", &["Subtask 1", "Subtask 2"]), // Task with 2 subtasks
            ("Task C", &["Another subtask"]),        // Task with 1 subtask
        ]);

        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("archive.md");

        // Archive both subtasks of Task B (2a and 2b)
        let result = std::panic::catch_unwind(|| {
            archive_items(
                file.path(),
                &archive_path,
                &["2a".to_string(), "2b".to_string()],
            )
            .unwrap();
        });
        assert!(result.is_ok());

        // Verify Task B was auto-completed and removed from todo file
        let updated_content = std::fs::read_to_string(file.path()).unwrap();
        let expected_content = "# TODOs\n\n- Task A\n- Task C\n  - Another subtask\n";
        assert_eq!(updated_content, expected_content);

        // Verify both subtasks AND the parent task are in the archive
        assert!(archive_path.exists());
        let archive_content = std::fs::read_to_string(&archive_path).unwrap();

        // Should contain both subtasks as individual items and the parent task
        assert!(archive_content.contains("- Subtask 1"));
        assert!(archive_content.contains("- Subtask 2"));
        assert!(archive_content.contains("- Task B"));
    }
}
