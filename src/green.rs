use libc;
use std::mem::size_of;
use std::ptr;

#[repr(C)]
struct Context {
    // callee save registers
    d8: u64,
    d9: u64,
    d10: u64,
    d11: u64,
    d12: u64,
    d13: u64,
    d14: u64,
    d15: u64,
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,

    x30: u64, // link register
    sp: u64,  // stack pointer

    stack: *mut libc::c_void,
}

type Callback = extern "C" fn() -> ();

extern "C" {
    pub fn asm_spawn(func: Callback) -> ();
}

const PAGE_SIZE: usize = 4 * 1024; // 4KiB

pub fn spawn(func: Callback, stack_size: usize) -> () {
    unsafe {
        // allocate context
        let ctx_ptr = libc::malloc(size_of::<Context>());
        let mut ctx = ctx_ptr as *mut Context;

        // allocate stack
        let mut stack: *mut libc::c_void = ptr::null_mut();
        let result =
            libc::posix_memalign(&mut stack as *mut *mut libc::c_void, stack_size, PAGE_SIZE);
        if result != 0 {
            panic!("posix_memalign");
        }
        (*ctx).stack = stack;

        // spawn green thread
        asm_spawn(func);

        // free stack and context
        libc::free(stack);
        libc::free(ctx_ptr);
    }
}
