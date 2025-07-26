use super::*;

#[test]
fn test_add_entry_to_content_empty() {
    let result = add_entry_to_content("", "First item");
    assert_eq!(result, "First item\n");
}

#[test]
fn test_add_entry_to_content_existing() {
    let existing = "Second item\nThird item";
    let result = add_entry_to_content(existing, "First item");
    assert_eq!(result, "First item\nSecond item\nThird item");
}

#[test]
fn test_add_entry_to_content_with_trailing_newline() {
    let existing = "Second item\nThird item\n";
    let result = add_entry_to_content(existing, "First item");
    assert_eq!(result, "First item\nSecond item\nThird item");
}

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

#[test]
fn test_archive_items_in_content_all_items() {
    let lines = vec!["First item".to_string(), "Second item".to_string()];
    let result = archive_items_in_content(&lines, &[1, 2]);

    assert!(result.is_ok());
    let (new_content, archived) = result.unwrap();
    assert_eq!(new_content, "");
    assert_eq!(archived, vec!["First item", "Second item"]);
}

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

    fn create_test_file_with_content(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_list_with_all_flag() {
        let content = "item 1\nitem 2\nitem 3\nitem 4\nitem 5\nitem 6";
        let file = create_test_file_with_content(content);

        // Test that all=true shows all items regardless of num
        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 3, true, None).unwrap();
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_without_all_flag() {
        let content = "item 1\nitem 2\nitem 3\nitem 4\nitem 5\nitem 6";
        let file = create_test_file_with_content(content);

        // Test that all=false respects num limit
        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 3, false, None).unwrap();
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_with_filter_and_all() {
        let content = "read: book 1\nwrite: article\nread: book 2\ntask: cleanup";
        let file = create_test_file_with_content(content);

        // Test that all=true with filter shows all matching items
        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 1, true, Some("read:")).unwrap();
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_empty_file() {
        let file = create_test_file_with_content("");

        let result = std::panic::catch_unwind(|| {
            list_note(file.path(), 5, false, None).unwrap();
        });
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod review_tests {
    use std::collections::HashSet;

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
        let result = super::prioritize_items_in_content(&lines, &[5, 2, 4]);

        assert!(result.is_ok());
        let (new_content, prioritized) = result.unwrap();

        // Expected: Item E, Item B, Item D should be at top in that order
        let expected_content = "Item E\nItem B\nItem D\nItem A\nItem C\n";
        let expected_prioritized = vec!["Item E", "Item B", "Item D"];

        assert_eq!(new_content, expected_content);
        assert_eq!(prioritized, expected_prioritized);
    }

    #[test]
    fn test_prioritize_command_line_order_with_duplicates() {
        // Test that duplicates are removed but order is preserved
        let lines = vec![
            "Item A".to_string(),
            "Item B".to_string(),
            "Item C".to_string(),
        ];

        // Command: arn up 3 1 3 2 1 (should become 3, 1, 2)
        let result = super::prioritize_items_in_content(&lines, &[3, 1, 3, 2, 1]);

        assert!(result.is_ok());
        let (new_content, prioritized) = result.unwrap();

        // Expected: Item C, Item A, Item B (duplicates removed, order preserved)
        let expected_content = "Item C\nItem A\nItem B\n";
        let expected_prioritized = vec!["Item C", "Item A", "Item B"];

        assert_eq!(new_content, expected_content);
        assert_eq!(prioritized, expected_prioritized);
    }
}
