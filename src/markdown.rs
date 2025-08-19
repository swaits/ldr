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

/// Represents a named list of tasks
#[derive(Debug, Clone, PartialEq)]
pub struct TaskList {
    pub name: String,
    pub tasks: Vec<Task>,
}

impl TaskList {
    pub fn new(name: String) -> Self {
        TaskList {
            name,
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

    #[allow(dead_code)]
    pub fn total_item_count(&self) -> usize {
        self.tasks
            .iter()
            .map(|t| 1 + t.subtask_count())
            .sum()
    }
}

/// Represents the entire todo file with multiple lists
#[derive(Debug, Clone, PartialEq)]
pub struct TodoFile {
    pub title: String,
    pub lists: Vec<TaskList>,
}

impl TodoFile {
    pub fn new(title: String) -> Self {
        TodoFile {
            title,
            lists: Vec::new(),
        }
    }

    pub fn add_list(&mut self, list: TaskList) {
        self.lists.push(list);
    }

    pub fn get_default_list_mut(&mut self) -> Option<&mut TaskList> {
        self.lists.iter_mut().find(|list| list.name == "Default")
    }

    pub fn get_default_list(&self) -> Option<&TaskList> {
        self.lists.iter().find(|list| list.name == "Default")
    }

    pub fn get_list_mut(&mut self, name: &str) -> Option<&mut TaskList> {
        self.lists.iter_mut().find(|list| list.name == name)
    }

    pub fn get_list(&self, name: &str) -> Option<&TaskList> {
        self.lists.iter().find(|list| list.name == name)
    }

    pub fn ensure_default_list(&mut self) {
        if self.get_default_list().is_none() {
            self.lists.insert(0, TaskList::new("Default".to_string()));
        }
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
                    return Err(format!("Task reference must start with a number: {}", input));
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

        let task_index = task_part.parse::<usize>().map_err(|_| {
            format!("Invalid task number: {}", task_part)
        })? - 1; // Convert to 0-based

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

/// Parse a markdown todo file
pub fn parse_todo_file(content: &str) -> Result<TodoFile, String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(TodoFile::new("TODOs".to_string()));
    }

    let mut todo_file = TodoFile::new("TODOs".to_string());
    let mut current_list: Option<TaskList> = None;
    let mut current_task: Option<Task> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some(title) = line.strip_prefix("# ") {
            todo_file.title = title.to_string();
        } else if let Some(list_name) = line.strip_prefix("## ") {
            // Save previous task if exists
            if let (Some(mut list), Some(task)) = (current_list.take(), current_task.take()) {
                list.add_task(task);
                todo_file.add_list(list);
            } else if let Some(list) = current_list.take() {
                todo_file.add_list(list);
            }

            current_list = Some(TaskList::new(list_name.to_string()));
        } else if let Some(subtask_text) = line.strip_prefix("  - ") {
            if let Some(ref mut task) = current_task {
                task.add_subtask(subtask_text.to_string());
            } else {
                return Err(format!(
                    "Subtask found without parent task at line {}: {}",
                    line_num + 1,
                    line
                ));
            }
        } else if let Some(task_text) = line.strip_prefix("- ") {
            // Save previous task if exists
            if let (Some(list), Some(task)) = (current_list.as_mut(), current_task.take()) {
                list.add_task(task);
            }

            current_task = Some(Task::new(task_text.to_string()));
        } else if line.starts_with("    - ") {
            return Err(format!(
                "Deep nesting not allowed. Only single-level subtasks are supported at line {}: {}",
                line_num + 1,
                line
            ));
        } else if !trimmed.is_empty() {
            return Err(format!(
                "Invalid markdown format at line {}: {}",
                line_num + 1,
                line
            ));
        }
    }

    // Save final task and list
    if let Some(mut list) = current_list {
        if let Some(task) = current_task {
            list.add_task(task);
        }
        todo_file.add_list(list);
    }

    // Ensure there's always a Default list
    todo_file.ensure_default_list();

    Ok(todo_file)
}

/// Generate markdown content from a TodoFile
pub fn generate_todo_file(todo_file: &TodoFile) -> String {
    let mut content = String::new();
    content.push_str(&format!("# {}\n\n", todo_file.title));

    for list in &todo_file.lists {
        content.push_str(&format!("## {}\n", list.name));

        for task in &list.tasks {
            content.push_str(&format!("- {}\n", task.text));
            for subtask in &task.subtasks {
                content.push_str(&format!("  - {}\n", subtask));
            }
        }
        content.push('\n');
    }

    content.trim_end().to_string() + "\n"
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
        
        // Find or create today's entry
        let entry = self.entries.iter_mut()
            .find(|e| e.date == today)
            .map(|e| e as *mut ArchiveEntry);

        if let Some(entry) = entry {
            unsafe {
                (*entry).lists
                    .entry(list_name.to_string())
                    .or_insert_with(Vec::new)
                    .extend(tasks);
            }
        } else {
            let mut lists = HashMap::new();
            lists.insert(list_name.to_string(), tasks);
            self.entries.insert(0, ArchiveEntry {
                date: today,
                lists,
            });
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
                entry.lists
                    .entry(current_list_name.clone())
                    .or_insert_with(Vec::new)
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
            if let (Some(ref mut entry), Some(task)) = (current_entry.as_mut(), current_task.take()) {
                entry.lists
                    .entry(current_list_name.clone())
                    .or_insert_with(Vec::new)
                    .push(task);
            }

            current_list_name = list_name.to_string();
        } else if let Some(task_text) = trimmed.strip_prefix("- ") {
            // Save previous task
            if let (Some(ref mut entry), Some(task)) = (current_entry.as_mut(), current_task.take()) {
                entry.lists
                    .entry(current_list_name.clone())
                    .or_insert_with(Vec::new)
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
            entry.lists
                .entry(current_list_name)
                .or_insert_with(Vec::new)
                .push(task);
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
    }

    #[test]
    fn test_parse_simple_todo_file() {
        let content = r#"# TODOs

## Default
- First task
- Second task
  - Subtask A
  - Subtask B

## Work
- Work task
"#;

        let todo_file = parse_todo_file(content).unwrap();
        assert_eq!(todo_file.title, "TODOs");
        assert_eq!(todo_file.lists.len(), 2);

        let default_list = todo_file.get_list("Default").unwrap();
        assert_eq!(default_list.tasks.len(), 2);
        assert_eq!(default_list.tasks[1].subtasks.len(), 2);
    }

    #[test]
    fn test_generate_todo_file() {
        let mut todo_file = TodoFile::new("TODOs".to_string());
        let mut default_list = TaskList::new("Default".to_string());
        
        let mut task = Task::new("Task with subtasks".to_string());
        task.add_subtask("Subtask 1".to_string());
        task.add_subtask("Subtask 2".to_string());
        default_list.add_task(task);
        
        todo_file.add_list(default_list);

        let generated = generate_todo_file(&todo_file);
        let expected = r#"# TODOs

## Default
- Task with subtasks
  - Subtask 1
  - Subtask 2
"#;
        assert_eq!(generated, expected);
    }

    #[test]
    fn test_reject_deep_nesting() {
        let content = r#"# TODOs

## Default
- Task
  - Subtask
    - Deep subtask
"#;

        assert!(parse_todo_file(content).is_err());
    }
}