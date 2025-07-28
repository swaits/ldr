# LDR - Log, Do, Review

A simple command-line productivity system that lets you add items to the top of
your list and review them interactively.

## Inspiration

This tool was inspired by Andrej Karpathy's blog post ["The append-and-review
note"](https://karpathy.bearblog.dev/the-append-and-review-note/), which
describes a simple but effective productivity system using a single text file
where you append new items to the top and periodically review from the top down.

## Installation

### From crates.io

```bash
cargo install ldr
```

### Using mise

```bash
mise use -g cargo:ldr
```

### From source

1. Clone this repository
2. Build with Cargo:

   ```bash
   cargo build --release
   ```

3. Copy the binary to your PATH:

   ```bash
   cp target/release/ldr ~/.local/bin/
   ```

## Usage

- `ldr add "Your todo item"` - Add a new item to the top
- `ldr ls` - List the top 5 items (use `-n NUM` for different count, `-a` for all, or add filter text)
- `ldr scan` - Review items interactively
- `ldr do 1 2 3` - Archive completed items by number
- `ldr up 1 2 3` - Prioritize items by moving them to the top
- `ldr rm 1 2 3` - Remove items without archiving
- `ldr edit` - Edit your todo list in $EDITOR

### Command aliases

- `add` can also be used as `a` or `prepend`
- `ls` can also be used as `l` or `list`
- `up` can also be used as `u` or `prioritize`
- `do` can also be used as `d`, `done`, `finish`, or `check`
- `rm` can also be used as `remove`, `delete`, `destroy`, or `forget`
- `scan` can also be used as `s`, `r`, or `review`
- `edit` can also be used as `e`

## License

MIT License - see LICENSE file for details.
