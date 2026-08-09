use x86_64::registers::model_specific::{Efer, EferFlags, LStar, SFMask};
use x86_64::VirtAddr;

/// Assembly stub that saves registers, calls the rust handler, and restores.
/// Note: we use `global_asm!` which requires enabling the feature if not nightly,
/// but since we are on nightly, we can just use `core::arch::global_asm`.
use core::arch::global_asm;

// The syscall stub. It preserves user registers, calls `syscall_handler`, and uses `sysretq`.
global_asm!(r#"
.global syscall_entry
syscall_entry:
    // rcx contains user RIP
    // r11 contains user RFLAGS
    // For a real OS we need to swapgs and set up a kernel stack here!
    // For this prototype, we'll just preserve registers and call the rust handler.

    push rcx
    push r11
    push rdi
    push rsi
    push rdx
    push r8
    push r9
    push r10

    // sysv abi: args are in rdi, rsi, rdx, rcx (but r10 for syscalls), r8, r9
    mov rcx, r10 

    // Align stack
    mov rbp, rsp
    and rsp, -16
    
    call rust_syscall_handler
    
    mov rsp, rbp

    pop r10
    pop r9
    pop r8
    pop rdx
    pop rsi
    pop rdi
    pop r11
    pop rcx

    sysretq
"#);

unsafe extern "C" {
    fn syscall_entry();
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_syscall_handler(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64 {
    // Syscall numbers:
    // 1: sys_write (fd, *buf, len)
    // 60: sys_exit (code)
    match arg0 {
        1 => {
            // Very unsafe: directly reading user pointer
            // In a real OS, we must validate `arg2` (ptr) and `arg3` (len)
            let ptr = arg2 as *const u8;
            let len = arg3 as usize;
            
            // For demo: print to VGA
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            if let Ok(s) = core::str::from_utf8(slice) {
                crate::print!("{}", s);
                len as u64
            } else {
                (0u64).wrapping_sub(1) // -1
            }
        },
        60 => {
            crate::println!("Process exited with code {}", arg1);
            0
        },
        _ => {
            crate::println!("Unknown syscall: {}", arg0);
            (0u64).wrapping_sub(1) // -ENOSYS
        }
    }
}

pub fn init() {
    unsafe {
        // Enable syscalls (SCE bit in EFER)
        Efer::update(|efer| *efer |= EferFlags::SYSTEM_CALL_EXTENSIONS);
        
        let cs = crate::gdt::GDT.1.kernel_code_selector.0;
        let _ss = crate::gdt::GDT.1.kernel_data_selector.0;
        // User selectors for sysret. The hardware assumes user code is +16 from CS, user data +8 from CS (sysret assumes CS + 16, SS + 8).
        // Let's set the STAR MSR. 
        // 63:48 = user CS and SS. 
        // 47:32 = kernel CS and SS.
        let star_value = ((cs as u64) << 32) | (((cs as u64) + 16) << 48);
        x86_64::registers::model_specific::Msr::new(0xC0000081).write(star_value);

        // LSTAR contains the RIP for the syscall entry point
        LStar::write(VirtAddr::new(syscall_entry as usize as u64));

        // SFMASK masks out RFLAGS (e.g. disable interrupts on syscall entry)
        SFMask::write(x86_64::registers::rflags::RFlags::INTERRUPT_FLAG);
    }
}
