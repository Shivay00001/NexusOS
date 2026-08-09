#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};
use x86_64::VirtAddr;

mod vga_buffer;
pub mod interrupts;
pub mod gdt;
pub mod memory;
pub mod allocator;
pub mod task;
pub mod vfs;
pub mod shell;
pub mod editor;
pub mod pci;
pub mod syscall;
pub mod process;
pub mod net;
pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Phase 1: Hardware initialization
    init();

    // Phase 2: Memory management
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        memory::BitmapFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed");

    // Phase 3: Boot banner
    vga_buffer::WRITER.lock().clear_screen();
    println!("  _   _                       ___  ____  ");
    println!(" | \\ | | _____  ___   _ ___  / _ \\/ ___| ");
    println!(" |  \\| |/ _ \\ \\/ / | | / __|  | | \\___ \\ ");
    println!(" | |\\  |  __/>  <| |_| \\__ \\ |_| |___) |");
    println!(" |_| \\_|\\___/_/\\_\\\\__,_|___/\\___/|____/ ");
    println!("");
    println!(" NexusOS v0.2.0 | x86_64 | Rust Kernel");

    // Show memory stats
    let (total, used, free) = frame_allocator.stats();
    println!(" RAM: {} KiB total, {} KiB used, {} KiB free",
        total * 4, used * 4, free * 4);
    println!(" Heap: {} KiB | Allocator: Bitmap O(1)", allocator::HEAP_SIZE / 1024);
    // Phase 4: Hardware Subsystems (PCI & Syscalls)
    syscall::init();
    println!(" [Syscall] MSRs configured for Ring 3 transitions");
    
    let pci_devices = pci::scan_bus();
    println!(" [PCI] Scanned bus: Found {} devices", pci_devices.len());
    for dev in &pci_devices {
        if dev.vendor_id == 0x1AF4 && dev.device_id == 0x1000 {
            println!("   -> Virtio Network Device found at {:02x}:{:02x}.{}", dev.bus, dev.device, dev.function);
        }
    }
    println!("----------------------------------------------");
    println!(" Type 'help' for available commands.");
    println!("");

    // Phase 5: Create default filesystem structure
    {
        let mut fs = vfs::ROOT_FS.lock();
        let _ = fs.mkdir("home");
        let _ = fs.mkdir("etc");
        let _ = fs.mkdir("tmp");
        let _ = fs.create_file("/etc/hostname");
        let _ = fs.write_file("/etc/hostname", b"nexus\n");
        let _ = fs.create_file("/etc/version");
        let _ = fs.write_file("/etc/version", b"NexusOS 0.1.0\n");
    }

    // Phase 5: Launch async executor with keyboard/shell task
    let mut executor = task::simple_executor::SimpleExecutor::new();
    executor.spawn(task::Task::new(task::keyboard::print_keypresses()));
    executor.run();
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
