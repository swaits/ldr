# PARTS - Prepend and Review ToDo System

A simple command-line productivity system that lets you add items to the top of your list and review them interactively.

## Inspiration

This tool was inspired by Andrej Karpathy's blog post ["The append-and-review note"](https://karpathy.bearblog.dev/the-append-and-review-note/), which describes a simple but effective productivity system using a single text file where you append new items to the top and periodically review from the top down.

## Installation

1. Clone this repository
2. Build with Cargo:

   ```bash
   cargo build --release
   ```

3. Copy the binary to your PATH:

   ```bash
   cp target/release/parts ~/.local/bin/
   ```

## Usage

- `parts add "Your todo item"` - Add a new item to the top
- `parts ls` - List your items
- `parts scan` - Review items interactively
- `parts do 1 2 3` - Archive completed items by number
- `parts up 1 2 3` - Prioritize items by moving them to the top
- `parts edit` - Edit your todo list in $EDITOR

## License

MIT License - see LICENSE file for details.
