use x86_64::{
    structures::paging::{
        FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use alloc::vec::Vec;

/// Initialize a new OffsetPageTable.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
    unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// A bitmap-based frame allocator. O(1) allocation and deallocation.
///
/// Each bit in the bitmap represents one 4KiB physical frame.
/// 0 = free, 1 = used.
pub struct BitmapFrameAllocator {
    bitmap: Vec<u64>,      // Each u64 covers 64 frames (256 KiB)
    total_frames: usize,
    next_free: usize,      // Hint: index of the first possibly-free u64 word
    allocated_count: usize,
}

impl BitmapFrameAllocator {
    /// Create a BitmapFrameAllocator from the bootloader memory map.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        // Find the highest usable physical address to size the bitmap
        let max_addr = memory_map
            .iter()
            .filter(|r| r.region_type == MemoryRegionType::Usable)
            .map(|r| r.range.end_addr())
            .max()
            .unwrap_or(0);

        let total_frames = (max_addr as usize) / 4096;
        let bitmap_words = (total_frames + 63) / 64; // Round up to cover all frames

        // Start with all frames marked as USED (1)
        let mut bitmap = Vec::with_capacity(bitmap_words);
        for _ in 0..bitmap_words {
            bitmap.push(u64::MAX); // All bits set = all used
        }

        // Mark usable regions as FREE (0)
        for region in memory_map.iter() {
            if region.region_type == MemoryRegionType::Usable {
                let start_frame = region.range.start_addr() as usize / 4096;
                let end_frame = region.range.end_addr() as usize / 4096;
                for frame_idx in start_frame..end_frame {
                    if frame_idx < total_frames {
                        let word = frame_idx / 64;
                        let bit = frame_idx % 64;
                        bitmap[word] &= !(1u64 << bit); // Clear bit = mark free
                    }
                }
            }
        }

        BitmapFrameAllocator {
            bitmap,
            total_frames,
            next_free: 0,
            allocated_count: 0,
        }
    }

    /// Return statistics about memory usage.
    pub fn stats(&self) -> (usize, usize, usize) {
        let free = self.total_frames - self.allocated_count;
        (self.total_frames, self.allocated_count, free)
    }

    /// Free a frame (mark it as available again).
    pub fn free_frame(&mut self, frame: PhysFrame) {
        let frame_idx = frame.start_address().as_u64() as usize / 4096;
        if frame_idx < self.total_frames {
            let word = frame_idx / 64;
            let bit = frame_idx % 64;
            if self.bitmap[word] & (1u64 << bit) != 0 {
                self.bitmap[word] &= !(1u64 << bit);
                self.allocated_count -= 1;
                if word < self.next_free {
                    self.next_free = word;
                }
            }
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Scan from next_free hint for a word with at least one free bit
        for word_idx in self.next_free..self.bitmap.len() {
            let word = self.bitmap[word_idx];
            if word != u64::MAX {
                // Found a word with a free bit. Find the first zero bit.
                let bit = (!word).trailing_zeros() as usize;
                let frame_idx = word_idx * 64 + bit;
                if frame_idx >= self.total_frames {
                    return None;
                }
                // Mark as used
                self.bitmap[word_idx] |= 1u64 << bit;
                self.allocated_count += 1;
                self.next_free = word_idx; // Update hint

                let addr = PhysAddr::new((frame_idx * 4096) as u64);
                return Some(PhysFrame::containing_address(addr));
            }
        }
        None // Out of memory
    }
}
