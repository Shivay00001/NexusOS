use crate::{println, print};
use crate::vfs::ROOT_FS;
use crate::vga_buffer;
use alloc::string::ToString;

pub fn execute_command(cmd_line: &str) {
    let cmd_line = cmd_line.trim();
    if cmd_line.is_empty() {
        return;
    }

    let mut parts = cmd_line.split_whitespace();
    let command = parts.next().unwrap();

    match command {
        "echo" => {
            let text = parts.collect::<alloc::vec::Vec<&str>>().join(" ");
            println!("{}", text);
        }
        "clear" => {
            vga_buffer::WRITER.lock().clear_screen();
        }
        "ls" => {
            let path = parts.next().unwrap_or(".");
            let fs = ROOT_FS.lock();
            let lookup_path = if path == "." { fs.pwd().to_string() } else { path.to_string() };
            match fs.list_dir(&lookup_path) {
                Ok((dirs, files)) => {
                    if dirs.is_empty() && files.is_empty() {
                        println!("(empty)");
                    } else {
                        for d in &dirs {
                            // Print directories in different style
                            print!("{}/ ", d);
                        }
                        for f in &files {
                            print!("{} ", f);
                        }
                        if !dirs.is_empty() || !files.is_empty() {
                            println!("");
                        }
                    }
                }
                Err(e) => println!("ls: {}", e),
            }
        }
        "mkdir" => {
            if let Some(dirname) = parts.next() {
                let mut fs = ROOT_FS.lock();
                match fs.mkdir(dirname) {
                    Ok(()) => println!("Created directory: {}", dirname),
                    Err(e) => println!("mkdir: {}", e),
                }
            } else {
                println!("Usage: mkdir <dirname>");
            }
        }
        "rmdir" => {
            if let Some(dirname) = parts.next() {
                let mut fs = ROOT_FS.lock();
                match fs.rmdir(dirname) {
                    Ok(()) => println!("Removed directory: {}", dirname),
                    Err(e) => println!("rmdir: {}", e),
                }
            } else {
                println!("Usage: rmdir <dirname>");
            }
        }
        "cd" => {
            if let Some(path) = parts.next() {
                let mut fs = ROOT_FS.lock();
                match fs.cd(path) {
                    Ok(()) => {}
                    Err(e) => println!("cd: {}", e),
                }
            } else {
                println!("Usage: cd <path>");
            }
        }
        "pwd" => {
            let fs = ROOT_FS.lock();
            println!("{}", fs.pwd());
        }
        "touch" => {
            if let Some(filename) = parts.next() {
                let mut fs = ROOT_FS.lock();
                match fs.create_file(filename) {
                    Ok(()) => println!("Created file: {}", filename),
                    Err(e) => println!("touch: {}", e),
                }
            } else {
                println!("Usage: touch <filename>");
            }
        }
        "rm" => {
            if let Some(filename) = parts.next() {
                let mut fs = ROOT_FS.lock();
                match fs.delete_file(filename) {
                    Ok(()) => println!("Deleted: {}", filename),
                    Err(e) => println!("rm: {}", e),
                }
            } else {
                println!("Usage: rm <filename>");
            }
        }
        "write" => {
            if let Some(filename) = parts.next() {
                let text = parts.collect::<alloc::vec::Vec<&str>>().join(" ");
                if text.is_empty() {
                    println!("Usage: write <filename> <text>");
                    return;
                }
                let mut fs = ROOT_FS.lock();
                match fs.write_file(filename, text.as_bytes()) {
                    Ok(()) => {
                        let _ = fs.write_file(filename, b"\n");
                        println!("Written to {}", filename);
                    }
                    Err(e) => println!("write: {}", e),
                }
            } else {
                println!("Usage: write <filename> <text>");
            }
        }
        "cat" => {
            if let Some(filename) = parts.next() {
                let fs = ROOT_FS.lock();
                match fs.read_file(filename) {
                    Ok(data) => {
                        if let Ok(text) = core::str::from_utf8(data) {
                            print!("{}", text);
                        } else {
                            println!("<binary data, {} bytes>", data.len());
                        }
                    }
                    Err(e) => println!("cat: {}", e),
                }
            } else {
                println!("Usage: cat <filename>");
            }
        }
        "stat" => {
            if let Some(filename) = parts.next() {
                let fs = ROOT_FS.lock();
                match fs.file_info(filename) {
                    Ok((size,)) => {
                        println!("File: {}", filename);
                        println!("Size: {} bytes", size);
                    }
                    Err(e) => println!("stat: {}", e),
                }
            } else {
                println!("Usage: stat <filename>");
            }
        }
        "sysinfo" => {
            println!("=== NexusOS System Information ===");
            println!("Kernel:       NexusOS v0.2.0");
            println!("Architecture: x86_64 (64-bit)");
            println!("Language:     Rust (no_std)");
            println!("Heap Size:    {} KiB", crate::allocator::HEAP_SIZE / 1024);
            println!("Allocator:    Bitmap Frame Allocator (O(1))");
            println!("VGA Mode:     80x25 text");
            println!("Interrupts:   PIC 8259 (Timer + Keyboard)");
            println!("Scheduler:    Cooperative Async/Await");
            println!("Filesystem:   Ramfs (hierarchical, in-memory)");
            println!("Editor:       Built-in line editor");
        }
        "uname" => {
            println!("NexusOS 0.2.0 x86_64");
        }
        "help" => {
            println!("=== NexusOS Shell Commands ===");
            println!("");
            println!("  Filesystem:");
            println!("    ls [path]             - List directory contents");
            println!("    cd <path>             - Change directory");
            println!("    pwd                   - Print working directory");
            println!("    mkdir <dir>           - Create directory");
            println!("    rmdir <dir>           - Remove empty directory");
            println!("    touch <file>          - Create file");
            println!("    rm <file>             - Delete file");
            println!("    write <file> <text>   - Append text to file");
            println!("    cat <file>            - Read file contents");
            println!("    stat <file>           - Show file info");
            println!("");
            println!("  Editor:");
            println!("    edit <file>           - Open text editor");
            println!("      :w                  - Save");
            println!("      :q                  - Quit (or :q! to force)");
            println!("      :wq                 - Save and quit");
            println!("      :d N               - Delete line N");
            println!("");
            println!("  System:");
            println!("    echo <text>           - Print text");
            println!("    clear                 - Clear screen");
            println!("    sysinfo               - System information");
            println!("    uname                 - OS version");
            println!("    help                  - This help message");
        }
        _ => {
            println!("Unknown command: {}", command);
            println!("Type 'help' for available commands.");
        }
    }
}
