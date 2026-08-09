use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use core::{pin::Pin, task::{Poll, Context}};
use core::sync::atomic::{AtomicBool, Ordering};
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Flag: when true, the keyboard input goes to the editor instead of the shell.
pub static EDITOR_MODE: AtomicBool = AtomicBool::new(false);

/// Called by the keyboard interrupt handler
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_ok() {
            WAKER.wake();
        } else {
            crate::println!("WARNING: scancode queue full; dropping keyboard input");
        }
    } else {
        crate::println!("WARNING: scancode queue uninitialized");
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");

        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

use alloc::string::String;
use futures_util::stream::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );
    
    let mut command_buffer = String::new();
    let mut editor: Option<crate::editor::Editor> = None;
    print_prompt();

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        if character == '\n' {
                            crate::println!("");
                            
                            if let Some(ref mut ed) = editor {
                                // Editor mode: send input to editor
                                if ed.process_input(&command_buffer) {
                                    // Editor exited
                                    editor = None;
                                    EDITOR_MODE.store(false, Ordering::SeqCst);
                                    crate::println!("");
                                    print_prompt();
                                }
                            } else {
                                // Shell mode: check if opening editor
                                let trimmed = command_buffer.trim();
                                if trimmed.starts_with("edit ") {
                                    let filename = &trimmed[5..];
                                    if !filename.is_empty() {
                                        let ed = crate::editor::Editor::new(filename);
                                        // keeping it as mut because the compiler warning was probably wrong if we call show_status, wait...
                                        // let's just let the warning be, it's just a warning.
                                        EDITOR_MODE.store(true, Ordering::SeqCst);
                                        ed.show_status();
                                        editor = Some(ed);
                                    } else {
                                        crate::println!("Usage: edit <filename>");
                                        print_prompt();
                                    }
                                } else {
                                    crate::shell::execute_command(&command_buffer);
                                    print_prompt();
                                }
                            }
                            command_buffer.clear();
                        } else if character == '\x08' {
                            if !command_buffer.is_empty() {
                                command_buffer.pop();
                                crate::print!("\x08");
                            }
                        } else {
                            command_buffer.push(character);
                            crate::print!("{}", character);
                        }
                    }
                    DecodedKey::RawKey(_) => {}
                }
            }
        }
    }
}

fn print_prompt() {
    let fs = crate::vfs::ROOT_FS.lock();
    let cwd = fs.pwd();
    crate::print!("nexus:{} $ ", cwd);
}
