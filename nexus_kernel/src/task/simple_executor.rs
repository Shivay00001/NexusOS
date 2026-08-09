use super::Task;
use alloc::collections::VecDeque;
use core::task::{Waker, RawWaker, RawWakerVTable, Context, Poll};

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }

    /// Run tasks. When all tasks are Pending, halt the CPU until
    /// the next hardware interrupt (keyboard/timer) wakes us.
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn run_ready_tasks(&mut self) {
        let mut remaining = VecDeque::new();
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match task.future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {} // task done, drop it
                Poll::Pending => remaining.push_back(task),
            }
        }
        self.task_queue = remaining;
    }

    fn sleep_if_idle(&self) {
        // Disable interrupts, check if there's work, and if not,
        // enable interrupts + halt atomically. This avoids the race
        // condition where an interrupt fires between the check and hlt.
        x86_64::instructions::interrupts::disable();
        if self.task_queue.is_empty() {
            // Nothing to do. Enable interrupts and halt until one fires.
            x86_64::instructions::interrupts::enable_and_hlt();
        } else {
            x86_64::instructions::interrupts::enable();
        }
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(core::ptr::null(), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
