use libc;
use std::collections::LinkedList;
use std::mem::size_of;
use std::ptr;

type Entry = fn();

#[repr(C)]
struct Registers {
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
}

#[repr(C)]
struct Context {
    regs: Registers,
    stack: *mut libc::c_void,
    entry: Entry,
}

impl Context {
    fn get_regs(&mut self) -> *mut Registers {
        &mut self.regs as *mut Registers
    }
}

extern "C" {
    fn set_context(ctx: *mut Registers) -> u64;
    fn switch_context(ctx: *mut Registers) -> !;
}

const PAGE_SIZE: usize = 4 * 1024; // 4KiB

static mut CTX_MAIN: *mut Context = ptr::null_mut();
static mut UNUSED_STACK: *mut libc::c_void = ptr::null_mut();
static mut CONTEXTS: LinkedList<*mut Context> = LinkedList::new();

pub fn spawn_from_main(func: Entry, stack_size: usize) {
    unsafe {
        if CTX_MAIN != ptr::null_mut() {
            panic!("spawn_from_main is called again before clean up");
        }

        CTX_MAIN = libc::malloc(size_of::<Context>()) as *mut Context;
        if set_context((*CTX_MAIN).get_regs()) == 0 {
            let ctx = alloc_ctx(func, stack_size);
            CONTEXTS.push_back(ctx);
            switch_context((*ctx).get_regs());
        } else {
            rm_unused_stack();
            libc::free(CTX_MAIN as *mut libc::c_void);
            CTX_MAIN = ptr::null_mut();
        }
    }
}

pub fn spawn(func: Entry, stack_size: usize) -> () {
    unsafe {
        let ctx = alloc_ctx(func, stack_size);
        CONTEXTS.push_back(ctx);
        schedule();
    }
}

pub fn schedule() {
    unsafe {
        if CONTEXTS.len() == 1 {
            return;
        }

        let ctx = CONTEXTS.pop_front().unwrap();
        CONTEXTS.push_back(ctx);
        if set_context((*ctx).get_regs()) == 0 {
            let next = CONTEXTS.front().unwrap();
            switch_context((**next).get_regs());
        } else {
            rm_unused_stack();
        }
    }
}

unsafe fn alloc_ctx(func: Entry, stack_size: usize) -> *mut Context {
    // allocate context
    let ctx_ptr = libc::malloc(size_of::<Context>());
    let mut ctx = ctx_ptr as *mut Context;

    // allocate stack
    let mut stack: *mut libc::c_void = ptr::null_mut();
    let result = libc::posix_memalign(&mut stack as *mut *mut libc::c_void, PAGE_SIZE, stack_size);
    if result != 0 {
        panic!("failed posix_memalign");
    }

    (*ctx).regs.x30 = entry_point as u64;
    (*ctx).regs.sp = stack as u64 + stack_size as u64;
    (*ctx).stack = stack;
    (*ctx).entry = func;

    ctx
}

extern "C" fn entry_point() {
    unsafe {
        let ctx = CONTEXTS.front().unwrap();
        ((**ctx).entry)();

        let ctx = CONTEXTS.pop_front().unwrap();
        UNUSED_STACK = (*ctx).stack;
        libc::free(ctx as *mut libc::c_void);

        let ctx = match CONTEXTS.front() {
            Some(c) => *c,
            None => CTX_MAIN,
        };
        switch_context((*ctx).get_regs());
    }
}

unsafe fn rm_unused_stack() {
    if UNUSED_STACK != ptr::null_mut() {
        libc::free(UNUSED_STACK);
        UNUSED_STACK = ptr::null_mut();
    }
}
