//! # Paging
//! Some code was borrowed from [Phil Opp's Blog](http://os.phil-opp.com/modifying-page-tables.html)

use core::{mem, ptr};

use crate::device::cpu::registers::{control_regs, tlb};

use self::mapper::PageFlushAll;

pub use rmm::{
    Arch as RmmArch,
    Flusher,
    PageFlags,
    PhysicalAddress,
    TableKind,
    VirtualAddress,
};
pub use super::CurrentRmmArch as RmmA;

pub type PageMapper = rmm::PageMapper<RmmA, crate::arch::rmm::LockedAllocator>;
pub use crate::rmm::KernelMapper;

pub mod entry;
pub mod mapper;

/// Number of entries per page table
pub const ENTRY_COUNT: usize = RmmA::PAGE_ENTRIES;

/// Size of pages
pub const PAGE_SIZE: usize = RmmA::PAGE_SIZE;

/// Setup Memory Access Indirection Register
#[cold]
unsafe fn init_mair() {
    let mut val: control_regs::MairEl1 = control_regs::mair_el1();

    val.insert(control_regs::MairEl1::DEVICE_MEMORY);
    val.insert(control_regs::MairEl1::NORMAL_UNCACHED_MEMORY);
    val.insert(control_regs::MairEl1::NORMAL_WRITEBACK_MEMORY);

    control_regs::mair_el1_write(val);
}

/// Initialize MAIR
#[cold]
pub unsafe fn init() {
    init_mair();
}

/// Page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress::new(self.number * PAGE_SIZE)
    }

    pub fn p4_index(self) -> usize {
        (self.number >> 27) & 0o777
    }

    pub fn p3_index(self) -> usize {
        (self.number >> 18) & 0o777
    }

    pub fn p2_index(self) -> usize {
        (self.number >> 9) & 0o777
    }

    pub fn p1_index(self) -> usize {
        self.number & 0o777
    }

    pub fn containing_address(address: VirtualAddress) -> Page {
        //TODO assert!(address.data() < 0x0000_8000_0000_0000 || address.data() >= 0xffff_8000_0000_0000,
        //    "invalid address: 0x{:x}", address.data());
        Page {
            number: address.data() / PAGE_SIZE,
        }
    }

    pub fn range_inclusive(start: Page, r#final: Page) -> PageIter {
        PageIter { start, end: r#final.next() }
    }
    pub fn range_exclusive(start: Page, end: Page) -> PageIter {
        PageIter { start, end }
    }

    pub fn next(self) -> Page {
        self.next_by(1)
    }
    pub fn next_by(self, n: usize) -> Page {
        Self {
            number: self.number + n,
        }
    }
    pub fn offset_from(self, other: Self) -> usize {
        self.number - other.number
    }
}

pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start < self.end {
            let page = self.start;
            self.start = self.start.next();
            Some(page)
        } else {
            None
        }
    }
}

/// Round down to the nearest multiple of page size
pub fn round_down_pages(number: usize) -> usize {
    number - number % PAGE_SIZE
}
/// Round up to the nearest multiple of page size
pub fn round_up_pages(number: usize) -> usize {
    round_down_pages(number + PAGE_SIZE - 1)
}
