//! Markdown parsing and generation for todo files.
//!
//! This module handles reading and writing Markdown-formatted todo and archive files.
//! It supports single-level nesting (tasks with subtasks) and multiple named lists.

use std::collections::HashMap;

/// Represents a single task with optional subtasks
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub text: String,
    pub subtasks: Vec<String>,
}

impl Task {
    pub fn new(text: String) -> Self {
        Task {
            text,
            subtasks: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_subtasks(text: String, subtasks: Vec<String>) -> Self {
        Task { text, subtasks }
    }

    pub fn add_subtask(&mut self, subtask: String) {
        self.subtasks.push(subtask);
    }

    #[allow(dead_code)]
    pub fn has_subtasks(&self) -> bool {
        !self.subtasks.is_empty()
    }

    #[allow(dead_code)]
    pub fn subtask_count(&self) -> usize {
        self.subtasks.len()
    }
}

/// Represents the entire todo file with a single task list
#[derive(Debug, Clone, PartialEq)]
pub struct TodoFile {
    pub title: String,
    pub tasks: Vec<Task>,
}

impl TodoFile {
    pub fn new(title: String) -> Self {
        TodoFile {
            title,
            tasks: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn prepend_task(&mut self, task: Task) {
        self.tasks.insert(0, task);
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

/// Parse task reference in format "1", "2a", "10b", etc.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskRef {
    pub task_index: usize,
    pub subtask_index: Option<usize>,
}

impl TaskRef {
    pub fn parse(input: &str) -> Result<TaskRef, String> {
        if input.is_empty() {
            return Err("Empty task reference".to_string());
        }

        let chars: Vec<char> = input.chars().collect();
        let mut task_part = String::new();
        let mut subtask_char = None;

        for (i, &ch) in chars.iter().enumerate() {
            if ch.is_ascii_digit() {
                task_part.push(ch);
            } else if ch.is_ascii_lowercase() {
                if i == 0 {
                    return Err(format!(
                        "Task reference must start with a number: {}",
                        input
                    ));
                }
                if subtask_char.is_some() {
                    return Err(format!("Multiple subtask letters not allowed: {}", input));
                }
                subtask_char = Some(ch);
            } else {
                return Err(format!("Invalid character in task reference: {}", ch));
            }
        }

        if task_part.is_empty() {
            return Err(format!("No task number found: {}", input));
        }

        let task_num = task_part
            .parse::<usize>()
            .map_err(|_| format!("Invalid task number: {}", task_part))?;

        // Validate task number range
        if task_num == 0 {
            return Err("Task number must be at least 1".to_string());
        }

        const MAX_TASK_NUM: usize = 10000;
        if task_num > MAX_TASK_NUM {
            return Err(format!(
                "Task number too large: {}. Maximum is {}",
                task_num, MAX_TASK_NUM
            ));
        }

        let task_index = task_num - 1; // Convert to 0-based

        let subtask_index = subtask_char.map(|ch| (ch as usize) - ('a' as usize));

        Ok(TaskRef {
            task_index,
            subtask_index,
        })
    }

    #[allow(dead_code)]
    pub fn is_subtask(&self) -> bool {
        self.subtask_index.is_some()
    }
}

/// Parse a markdown todo file with resilient handling of user edits
pub fn parse_todo_file(content: &str) -> Result<TodoFile, String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(TodoFile::new("TODOs".to_string()));
    }

    let mut todo_file = TodoFile::new("TODOs".to_string());
    let mut current_task: Option<Task> = None;
    let mut warned_about_deep_nesting = false;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Handle title - be flexible with spacing
        if let Some(title) = trimmed.strip_prefix("# ") {
            todo_file.title = title.trim().to_string();
        } else if trimmed.starts_with("#") && !trimmed.starts_with("##") {
            // Handle cases where user might not have space after # (but not ##)
            let title = &trimmed[1..];
            todo_file.title = title.trim().to_string();
        }
        // Skip any list headers - we ignore them now since we don't support multiple lists
        else if trimmed.starts_with("##") {
            continue;
        }
        // Check for deep nesting first - warn and convert to level 1 subtask
        else if (line.starts_with("     ") || line.starts_with("\t\t"))
            && (line.trim_start().starts_with("- ")
                || line.trim_start().starts_with("* ")
                || line.trim_start().starts_with("+ "))
        {
            // This is a deeply nested item - warn once and treat as level 1 subtask
            if !warned_about_deep_nesting {
                eprintln!("Warning: Deep nesting detected (line {}). Converting to level 1 subtask. Only single-level subtasks are supported.", line_num + 1);
                warned_about_deep_nesting = true;
            }

            let item_text = line
                .trim_start()
                .strip_prefix("- ")
                .or_else(|| line.trim_start().strip_prefix("* "))
                .or_else(|| line.trim_start().strip_prefix("+ "))
                .unwrap_or(line.trim_start());

            if let Some(ref mut task) = current_task {
                task.add_subtask(item_text.trim().to_string());
            } else {
                // If no current task, treat as main task
                current_task = Some(Task::new(item_text.trim().to_string()));
            }
        }
        // Handle subtasks - be flexible with indentation (2, 3, or 4 spaces, or single tab)
        else if let Some(subtask_text) = line.strip_prefix("  - ") {
            if let Some(ref mut task) = current_task {
                task.add_subtask(subtask_text.trim().to_string());
            } else {
                // If no current task, treat as main task (user error but be forgiving)
                current_task = Some(Task::new(subtask_text.trim().to_string()));
            }
        } else if let Some(subtask_text) = line.strip_prefix("   - ") {
            // Handle 3-space indentation
            if let Some(ref mut task) = current_task {
                task.add_subtask(subtask_text.trim().to_string());
            } else {
                current_task = Some(Task::new(subtask_text.trim().to_string()));
            }
        } else if let Some(subtask_text) = line.strip_prefix("    - ") {
            // Handle 4-space indentation
            if let Some(ref mut task) = current_task {
                task.add_subtask(subtask_text.trim().to_string());
            } else {
                current_task = Some(Task::new(subtask_text.trim().to_string()));
            }
        } else if let Some(subtask_text) = line.strip_prefix("\t- ") {
            // Handle single tab indentation
            if let Some(ref mut task) = current_task {
                task.add_subtask(subtask_text.trim().to_string());
            } else {
                current_task = Some(Task::new(subtask_text.trim().to_string()));
            }
        }
        // Handle main tasks - flexible with spacing and different bullet styles
        else if let Some(task_text) = trimmed.strip_prefix("- ") {
            // Save previous task if exists
            if let Some(task) = current_task.take() {
                todo_file.add_task(task);
            }

            current_task = Some(Task::new(task_text.trim().to_string()));
        } else if let Some(task_text) = trimmed.strip_prefix("* ") {
            // Handle asterisk bullet points
            if let Some(task) = current_task.take() {
                todo_file.add_task(task);
            }

            current_task = Some(Task::new(task_text.trim().to_string()));
        } else if let Some(task_text) = trimmed.strip_prefix("+ ") {
            // Handle plus bullet points
            if let Some(task) = current_task.take() {
                todo_file.add_task(task);
            }

            current_task = Some(Task::new(task_text.trim().to_string()));
        }
        // Handle non-markdown lines gracefully - ignore unknown formatting
        else if !trimmed.is_empty() {
            // If it looks like it might be a task without proper formatting, treat it as one
            if !trimmed.starts_with('#') && !trimmed.starts_with('<') && !trimmed.contains("```") {
                // Save previous task if exists
                if let Some(task) = current_task.take() {
                    todo_file.add_task(task);
                }

                current_task = Some(Task::new(trimmed.to_string()));
            }
            // Otherwise just skip unknown lines (comments, HTML, code blocks, etc.)
        }
    }

    // Save final task
    if let Some(task) = current_task {
        todo_file.add_task(task);
    }

    Ok(todo_file)
}

/// Generate markdown content from a TodoFile
pub fn generate_todo_file(todo_file: &TodoFile) -> String {
    let mut content = String::new();
    content.push_str(&format!("# {}\n\n", todo_file.title));

    for task in &todo_file.tasks {
        content.push_str(&format!("- {}\n", task.text));
        for subtask in &task.subtasks {
            content.push_str(&format!("  - {}\n", subtask));
        }
    }

    content
}

/// Parse an archive file with date-based sections
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveFile {
    pub title: String,
    pub entries: Vec<ArchiveEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveEntry {
    pub date: String,
    pub lists: HashMap<String, Vec<Task>>,
}

impl ArchiveFile {
    pub fn new() -> Self {
        ArchiveFile {
            title: "Archive".to_string(),
            entries: Vec::new(),
        }
    }

    pub fn add_items_for_today(&mut self, list_name: &str, tasks: Vec<Task>) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        // Find today's entry position
        let entry_pos = self.entries.iter().position(|e| e.date == today);

        if let Some(pos) = entry_pos {
            // Entry exists, add tasks to it
            self.entries[pos]
                .lists
                .entry(list_name.to_string())
                .or_default()
                .extend(tasks);
        } else {
            // Create new entry for today
            let mut lists = HashMap::new();
            lists.insert(list_name.to_string(), tasks);
            self.entries.insert(0, ArchiveEntry { date: today, lists });
        }
    }
}

/// Parse an archive file
pub fn parse_archive_file(content: &str) -> Result<ArchiveFile, String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(ArchiveFile::new());
    }

    let mut archive = ArchiveFile::new();
    let mut current_entry: Option<ArchiveEntry> = None;
    let mut current_list_name = "Default".to_string();
    let mut current_task: Option<Task> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("# ") {
            archive.title = title.to_string();
        } else if let Some(date) = trimmed.strip_prefix("## ") {
            // Save previous task and entry
            if let (Some(mut entry), Some(task)) = (current_entry.take(), current_task.take()) {
                entry
                    .lists
                    .entry(current_list_name.clone())
                    .or_default()
                    .push(task);
                archive.entries.push(entry);
            } else if let Some(entry) = current_entry.take() {
                archive.entries.push(entry);
            }

            current_entry = Some(ArchiveEntry {
                date: date.to_string(),
                lists: HashMap::new(),
            });
            current_list_name = "Default".to_string();
        } else if let Some(list_name) = trimmed.strip_prefix("### ") {
            // Save previous task
            if let (Some(ref mut entry), Some(task)) = (current_entry.as_mut(), current_task.take())
            {
                entry
                    .lists
                    .entry(current_list_name.clone())
                    .or_default()
                    .push(task);
            }

            current_list_name = list_name.to_string();
        } else if let Some(task_text) = trimmed.strip_prefix("- ") {
            // Save previous task
            if let (Some(ref mut entry), Some(task)) = (current_entry.as_mut(), current_task.take())
            {
                entry
                    .lists
                    .entry(current_list_name.clone())
                    .or_default()
                    .push(task);
            }

            current_task = Some(Task::new(task_text.to_string()));
        } else if let Some(subtask_text) = trimmed.strip_prefix("  - ") {
            if let Some(ref mut task) = current_task {
                task.add_subtask(subtask_text.to_string());
            } else {
                return Err(format!(
                    "Subtask found without parent task at line {}: {}",
                    line_num + 1,
                    trimmed
                ));
            }
        } else if !trimmed.is_empty() {
            return Err(format!(
                "Invalid archive format at line {}: {}",
                line_num + 1,
                trimmed
            ));
        }
    }

    // Save final task and entry
    if let Some(mut entry) = current_entry {
        if let Some(task) = current_task {
            entry.lists.entry(current_list_name).or_default().push(task);
        }
        archive.entries.push(entry);
    }

    Ok(archive)
}

/// Generate archive file content
pub fn generate_archive_file(archive: &ArchiveFile) -> String {
    let mut content = String::new();
    content.push_str(&format!("# {}\n\n", archive.title));

    for entry in &archive.entries {
        content.push_str(&format!("## {}\n", entry.date));

        // Add Default list items first (without header)
        if let Some(default_tasks) = entry.lists.get("Default") {
            for task in default_tasks {
                content.push_str(&format!("- {}\n", task.text));
                for subtask in &task.subtasks {
                    content.push_str(&format!("  - {}\n", subtask));
                }
            }
        }

        // Add other lists with headers
        for (list_name, tasks) in &entry.lists {
            if list_name != "Default" && !tasks.is_empty() {
                content.push_str(&format!("\n### {}\n", list_name));
                for task in tasks {
                    content.push_str(&format!("- {}\n", task.text));
                    for subtask in &task.subtasks {
                        content.push_str(&format!("  - {}\n", subtask));
                    }
                }
            }
        }
        content.push('\n');
    }

    content.trim_end().to_string() + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_ref_parsing() {
        assert_eq!(
            TaskRef::parse("1").unwrap(),
            TaskRef {
                task_index: 0,
                subtask_index: None
            }
        );

        assert_eq!(
            TaskRef::parse("5a").unwrap(),
            TaskRef {
                task_index: 4,
                subtask_index: Some(0)
            }
        );

        assert_eq!(
            TaskRef::parse("10z").unwrap(),
            TaskRef {
                task_index: 9,
                subtask_index: Some(25)
            }
        );

        assert!(TaskRef::parse("").is_err());
        assert!(TaskRef::parse("a").is_err());
        assert!(TaskRef::parse("1A").is_err());
        assert!(TaskRef::parse("1ab").is_err());
        assert!(TaskRef::parse("1-2").is_err());

        // Test validation edge cases
        assert!(TaskRef::parse("0").is_err()); // Zero task number
        assert!(TaskRef::parse("10001").is_err()); // Too large task number
        assert!(TaskRef::parse("999999999999999999999").is_err()); // Integer overflow
    }

    #[test]
    fn test_parse_simple_todo_file() {
        let content = r#"# TODOs

- First task
- Second task
  - Subtask A
  - Subtask B
- Third task
"#;

        let todo_file = parse_todo_file(content).unwrap();
        assert_eq!(todo_file.title, "TODOs");
        assert_eq!(todo_file.tasks.len(), 3);
        assert_eq!(todo_file.tasks[1].subtasks.len(), 2);
        assert_eq!(todo_file.tasks[1].subtasks[0], "Subtask A");
        assert_eq!(todo_file.tasks[1].subtasks[1], "Subtask B");
    }

    #[test]
    fn test_generate_todo_file() {
        let mut todo_file = TodoFile::new("TODOs".to_string());

        let mut task = Task::new("Task with subtasks".to_string());
        task.add_subtask("Subtask 1".to_string());
        task.add_subtask("Subtask 2".to_string());
        todo_file.add_task(task);

        let generated = generate_todo_file(&todo_file);
        let expected = r#"# TODOs

- Task with subtasks
  - Subtask 1
  - Subtask 2
"#;
        assert_eq!(generated, expected);
    }

    #[test]
    fn test_handle_deep_nesting_gracefully() {
        let content = r#"# TODOs

- Task
  - Subtask
    - Deep subtask
"#;

        // Resilient parser should ignore deep nesting rather than error
        let todo_file = parse_todo_file(content).unwrap();
        // The resilient parser treats 4 spaces as valid subtask indentation
        // So "Deep subtask" becomes a subtask rather than being ignored
        assert_eq!(todo_file.tasks.len(), 1);
        assert_eq!(todo_file.tasks[0].text, "Task");
        assert_eq!(todo_file.tasks[0].subtasks.len(), 2);
        assert_eq!(todo_file.tasks[0].subtasks[0], "Subtask");
        assert_eq!(todo_file.tasks[0].subtasks[1], "Deep subtask");
    }

    #[test]
    fn test_resilient_parsing_various_formats() {
        let content = r#"# TODOs

- Task with dash
* Task with asterisk
+ Task with plus
  - Subtask with 2 spaces
   - Subtask with 3 spaces
    - Subtask with 4 spaces
	- Subtask with tab

Plain text task without bullet
- Normal task

<!-- This is a comment -->
```
This is a code block
```
"#;

        let todo_file = parse_todo_file(content).unwrap();
        assert_eq!(todo_file.title, "TODOs");

        // Now all tasks are in the single task list
        assert_eq!(todo_file.tasks.len(), 6); // All tasks are top-level now (including code block content)

        // First task (dash) has no subtasks
        assert_eq!(todo_file.tasks[0].text, "Task with dash");
        assert_eq!(todo_file.tasks[0].subtasks.len(), 0);

        // Second task (asterisk) has no subtasks
        assert_eq!(todo_file.tasks[1].text, "Task with asterisk");
        assert_eq!(todo_file.tasks[1].subtasks.len(), 0);

        // Third task (plus) has all the subtasks (different indentation styles)
        assert_eq!(todo_file.tasks[2].text, "Task with plus");
        assert_eq!(todo_file.tasks[2].subtasks.len(), 4);

        // Plain text task and normal task
        assert_eq!(todo_file.tasks[3].text, "Plain text task without bullet");
        assert_eq!(todo_file.tasks[4].text, "Normal task");
        assert_eq!(todo_file.tasks[5].text, "This is a code block");
        // Comments are ignored but code block content is parsed
    }
}
