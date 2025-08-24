use std::process::Command;
use tempfile::TempDir;

fn run_ldr(dir: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ldr"))
        .env("XDG_DATA_HOME", dir.path())
        .args(args)
        .output()
        .expect("Failed to run ldr")
}

#[test]
fn test_empty_task_rejected() {
    let dir = TempDir::new().unwrap();

    // Try to add empty task
    let output = run_ldr(&dir, &["add", ""]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot add empty task"));

    // Try to add whitespace-only task
    let output = run_ldr(&dir, &["add", "   "]);
    assert!(!output.status.success());
}

#[test]
fn test_task_length_limit() {
    let dir = TempDir::new().unwrap();

    // Create a very long task (over 500 chars)
    let long_text = "x".repeat(501);
    let output = run_ldr(&dir, &["add", &long_text]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Task text too long"));
    assert!(stderr.contains("Maximum length is 500"));
}

#[test]
fn test_max_subtasks_limit() {
    let dir = TempDir::new().unwrap();

    // Add a main task
    run_ldr(&dir, &["add", "Main task"]);

    // Add 26 subtasks (a-z)
    for i in 0..26 {
        let subtask = format!("Subtask {}", i);
        let output = run_ldr(&dir, &["add", &subtask, "--under", "1"]);
        assert!(output.status.success());
    }

    // Try to add 27th subtask - should fail
    let output = run_ldr(&dir, &["add", "One too many", "--under", "1"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already has maximum number of subtasks"));
}

#[test]
fn test_invalid_task_references() {
    let dir = TempDir::new().unwrap();

    // Add some tasks
    run_ldr(&dir, &["add", "Task 1"]);
    run_ldr(&dir, &["add", "Task 2"]);

    // Test zero task number - should print error but return success
    let output = run_ldr(&dir, &["up", "0"]);
    // Check that error message was printed even if exit code is 0
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Invalid task reference")
            || stdout.contains("Task number must be at least 1")
    );

    // Test too large task number
    let output = run_ldr(&dir, &["up", "999"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid task number"));

    // Test invalid formats
    let output = run_ldr(&dir, &["up", "1A"]); // uppercase letter
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid") || stdout.contains("uppercase"));

    let output = run_ldr(&dir, &["up", "abc"]); // no number
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid") || stdout.contains("must start with a number"));
}

#[test]
fn test_subtask_under_invalid_parent() {
    let dir = TempDir::new().unwrap();

    // Try to add subtask under non-existent task
    let output = run_ldr(&dir, &["add", "Subtask", "--under", "1"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid task number"));
}

#[test]
fn test_deep_nesting_prevention() {
    let dir = TempDir::new().unwrap();

    // Create a markdown file with deep nesting
    // ldr stores files in a subdirectory
    std::fs::create_dir_all(dir.path().join("ldr")).unwrap();
    let todos_path = dir.path().join("ldr").join("todos.md");
    let content = r#"# TODOs

- Task 1
  - Subtask 1
    - Deep subtask (should be flattened)
      - Very deep subtask (should be flattened)
"#;
    std::fs::write(&todos_path, content).unwrap();

    // List tasks - deep nesting should be handled silently now
    // (warnings are printed during parse but list should work)
    let output = run_ldr(&dir, &["ls", "-a"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The deep subtasks should appear as regular subtasks (flattened)
    // We should see the subtasks in the output
    assert!(stdout.contains("Subtask 1") || stdout.contains("a."));
}
