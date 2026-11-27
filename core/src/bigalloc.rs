//! A global allocator that reserves large blocks without charging them to the commit limit.
//!
//! The module memory of the loader can be 4 GiB (65536 pages), of which a game touches a few MiB. Where the kernel
//! refuses a 4 GiB mapping by its heuristic overcommit rule, `MAP_NORESERVE` reserves the address range anyway and the
//! pages cost memory only when touched (docs/core-gate.md, "Known limits"). Everything below the threshold goes to the
//! system allocator.
use std::alloc::{GlobalAlloc, Layout, System};

const BIG: usize = 1 << 28;

pub struct BigAlloc;

unsafe fn syscall6(n: usize, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> isize {
    let ret: isize;
    std::arch::asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a, in("rsi") b, in("rdx") c,
        in("r10") d, in("r8") e, in("r9") f, lateout("rcx") _, lateout("r11") _, options(nostack));
    ret
}

unsafe fn map(size: usize) -> *mut u8 {
    // PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE
    let r = syscall6(9, 0, size, 3, 0x2 | 0x20 | 0x4000, usize::MAX, 0);
    if (-4095..0).contains(&r) {
        std::ptr::null_mut()
    } else {
        r as *mut u8
    }
}

unsafe impl GlobalAlloc for BigAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if l.size() >= BIG && l.align() <= 4096 {
            return map(l.size());
        }
        System.alloc(l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if l.size() >= BIG && l.align() <= 4096 {
            return map(l.size());
        }
        System.alloc_zeroed(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if l.size() >= BIG && l.align() <= 4096 {
            syscall6(11, p as usize, l.size(), 0, 0, 0, 0);
            return;
        }
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let big_old = l.size() >= BIG && l.align() <= 4096;
        let big_new = new >= BIG && l.align() <= 4096;
        if !big_old && !big_new {
            return System.realloc(p, l, new);
        }
        let n = self.alloc(Layout::from_size_align_unchecked(new, l.align()));
        if !n.is_null() {
            std::ptr::copy_nonoverlapping(p, n, l.size().min(new));
            self.dealloc(p, l);
        }
        n
    }
}
