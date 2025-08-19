//! Input handling utilities for interactive features.
//!
//! This module provides functions for reading raw keyboard input,
//! particularly for handling arrow keys in the interactive review mode.

use std::io::{self, Read};
use termion::raw::IntoRawMode;

/// Reads raw keyboard input including arrow keys and special characters.
/// Handles ANSI escape sequences for arrow keys and returns string representations.
/// Used for interactive navigation in review mode.
#[allow(dead_code)]
pub fn read_key_input() -> io::Result<String> {
    let _stdout = io::stdout().into_raw_mode()?;
    let mut buffer = [0; 3];

    // Read first byte
    io::stdin().read_exact(&mut buffer[0..1])?;

    match buffer[0] {
        27 => {
            // ESC sequence, read next two bytes
            io::stdin().read_exact(&mut buffer[1..3])?;
            match (buffer[1], buffer[2]) {
                (91, 65) => Ok("up".to_string()),    // ESC[A
                (91, 66) => Ok("down".to_string()),  // ESC[B
                (91, 67) => Ok("right".to_string()), // ESC[C
                (91, 68) => Ok("left".to_string()),  // ESC[D
                _ => Ok("unknown".to_string()),
            }
        }
        b'\n' | b'\r' => Ok("enter".to_string()),
        b'q' => Ok("q".to_string()),
        b'p' => Ok("p".to_string()),
        b'a' => Ok("a".to_string()),
        b's' => Ok("s".to_string()),
        _ => Ok("unknown".to_string()),
    }
}
