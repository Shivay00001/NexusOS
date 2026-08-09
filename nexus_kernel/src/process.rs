use x86_64::VirtAddr;
use core::arch::asm;
use crate::gdt;

/// A simple user process context.
pub struct Process {
    pub instruction_ptr: VirtAddr,
    pub stack_ptr: VirtAddr,
}

impl Process {
    /// Jump to user mode (Ring 3).
    /// This function never returns.
    pub unsafe fn jump_to_user_mode(&self) -> ! {
        // We use iretq to drop privileges from Ring 0 to Ring 3.
        // iretq expects the stack to be set up as follows:
        // [SS] [RSP] [RFLAGS] [CS] [RIP]
        
        let ds = gdt::GDT.1.user_data_selector.0;
        let cs = gdt::GDT.1.user_code_selector.0;
        
        // Ensure interrupts are enabled in the RFLAGS we push (bit 9)
        let rflags: u64 = 0x200 | (1 << 1); // IF | reserved bit 1

        unsafe {
            asm!(
                "push {ds}",        // Push SS (User Data Segment)
                "push {rsp}",       // Push RSP (User Stack)
                "push {rflags}",    // Push RFLAGS
                "push {cs}",        // Push CS (User Code Segment)
                "push {rip}",       // Push RIP (User Entry Point)
                "iretq",            // Return from interrupt to Ring 3!
                ds = in(reg) ds as u64,
                rsp = in(reg) self.stack_ptr.as_u64(),
                rflags = in(reg) rflags,
                cs = in(reg) cs as u64,
                rip = in(reg) self.instruction_ptr.as_u64(),
                options(noreturn)
            );
        }
    }
}
