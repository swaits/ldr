# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Core Design Principles
- **Principle of Least Surprise**: All UI behaviors should be predictable and intuitive
- **Dead Simple**: Complexity in storage, simplicity in interaction
- **Progressive Disclosure**: Basic operations stay simple, advanced features are optional
- **Backward Compatible**: Existing commands work unchanged on Default list

## Development Commands

- **Build project**: `cargo build`
- **Build release**: `cargo build --release`
- **Run locally**: `cargo run -- <subcommand> <args>` (e.g., `cargo run -- add "test item"`)
- **Run tests**: `cargo test`
- **Check code**: `cargo check`
- **Format code**: `cargo fmt`
- **Lint code**: `cargo clippy`
- **Install from source**: `cargo build --release && cp target/release/ldr ~/.local/bin/`

## Project Architecture

LDR is a Rust CLI productivity application implementing the "append-and-review" note system with Markdown format and subtask support. The architecture follows a clean separation of concerns:

### Core Structure
- **main.rs**: Entry point with CLI definition using `clap`. Handles XDG-compliant data directory setup (`~/.local/share/ldr/`) and automatic migration from plain text to Markdown format
- **commands.rs**: Command implementations with terminal I/O, file operations, and colored output using `termion`. Handles subtask operations and task reference parsing (1, 2a, 3b format)
- **markdown.rs**: Core data structures and parsing/generation for Markdown todo files. Supports single-level nesting (tasks with subtasks) and multiple named lists
- **migration.rs**: One-time migration utilities for converting plain text files to Markdown format with Default list structure
- **content.rs**: Legacy compatibility functions maintained for existing tests
- **input.rs**: Raw keyboard input handling for interactive review mode (currently simplified)

### File Format
- **todos.md**: Markdown file with multiple lists (Default list is primary)
- **archive.md**: Markdown archive with date-based sections and list organization
- **Migration**: Automatic one-time conversion from `note.txt`/`archive.txt` to Markdown format

### Task System
- **Task References**: Number+letter format (1, 2a, 3b) for referencing tasks and subtasks
- **Single-Level Nesting**: Tasks can have subtasks, but subtasks cannot have sub-subtasks
- **Default List**: Primary list for backward compatibility - all existing commands work on Default
- **Multiple Lists**: Support for named lists (Work, Personal, etc.) with `--list` flag

### Key Design Patterns
- **Markdown Storage**: Human-readable format with proper structure
- **Flat Numbering**: Simple number+letter system for task references
- **Backward Compatibility**: Existing interface unchanged, new features are additive
- **XDG Compliance**: Uses `xdg` crate for proper data directory handling
- **Migration on First Run**: Seamless upgrade from old format with backups

### Data Flow
1. CLI parsing extracts subcommands and arguments
2. Migration check and execution if needed (first run only)
3. XDG directories resolved for Markdown files (`todos.md`, `archive.md`)
4. Commands parse task references and delegate to Markdown operations
5. File I/O operations handle Markdown parsing/generation
6. Terminal output provides user feedback with colors and proper formatting

### Task Reference System
- **1**: Task 1 and all its subtasks
- **1a**: Only subtask 'a' of task 1
- **2b**: Only subtask 'b' of task 2
- Operations default to whole tasks, subtask references are specific