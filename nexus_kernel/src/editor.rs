use crate::{println, print};
use crate::vfs::ROOT_FS;
use crate::vga_buffer;
use alloc::string::String;
use alloc::vec::Vec;

/// A simple line-based text editor inside the OS.
/// Usage: edit <filename>
/// 
/// When inside the editor:
///   - Type text and press Enter to add lines
///   - Type `:w` to save and quit
///   - Type `:q` to quit without saving
///   - Type `:wq` to save and quit
///   - Type `:d <n>` to delete line n
pub struct Editor {
    filename: String,
    lines: Vec<String>,
    modified: bool,
}

impl Editor {
    pub fn new(filename: &str) -> Self {
        let mut lines = Vec::new();
        
        // Load existing file content if it exists
        let fs = ROOT_FS.lock();
        if let Ok(data) = fs.read_file(filename) {
            if let Ok(text) = core::str::from_utf8(data) {
                for line in text.lines() {
                    lines.push(String::from(line));
                }
            }
        }
        drop(fs);

        Editor {
            filename: String::from(filename),
            lines,
            modified: false,
        }
    }

    pub fn show_status(&self) {
        vga_buffer::WRITER.lock().clear_screen();
        println!("--- EDIT: {} ({} lines) ---", self.filename, self.lines.len());
        println!("");

        // Show lines with line numbers
        for (i, line) in self.lines.iter().enumerate() {
            println!(" {:>3} | {}", i + 1, line);
        }

        println!("");
        println!("--- :w save | :q quit | :wq save+quit | :d N delete line ---");
        print!("> ");
    }

    /// Process a line of input from the user.
    /// Returns true if the editor should exit.
    pub fn process_input(&mut self, input: &str) -> bool {
        let trimmed = input.trim();

        if trimmed == ":q" {
            if self.modified {
                println!("Unsaved changes! Use :q! to force quit or :wq to save.");
                print!("> ");
                return false;
            }
            return true;
        }

        if trimmed == ":q!" {
            return true;
        }

        if trimmed == ":w" {
            self.save();
            self.show_status();
            return false;
        }

        if trimmed == ":wq" {
            self.save();
            return true;
        }

        if trimmed.starts_with(":d ") {
            let num_str = trimmed[3..].trim();
            if let Ok(n) = num_str.parse::<usize>() {
                if n > 0 && n <= self.lines.len() {
                    self.lines.remove(n - 1);
                    self.modified = true;
                    self.show_status();
                } else {
                    println!("Invalid line number");
                    print!("> ");
                }
            } else {
                println!("Usage: :d <line_number>");
                print!("> ");
            }
            return false;
        }

        // Regular text: add as a new line
        self.lines.push(String::from(trimmed));
        self.modified = true;
        self.show_status();
        false
    }

    fn save(&mut self) {
        let mut fs = ROOT_FS.lock();
        // Create file if it doesn't exist
        let _ = fs.create_file(&self.filename);
        // Build content
        let mut content = String::new();
        for line in &self.lines {
            content.push_str(line);
            content.push('\n');
        }
        // We need to clear existing content first - delete and recreate
        let _ = fs.delete_file(&self.filename);
        let _ = fs.create_file(&self.filename);
        let _ = fs.write_file(&self.filename, content.as_bytes());
        self.modified = false;
        println!("Saved {} ({} bytes)", self.filename, content.len());
    }
}
