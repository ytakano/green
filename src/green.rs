use libc;
use std::collections::LinkedList;
use std::ptr;

type Entry = fn();

const PAGE_SIZE: usize = 4 * 1024; // 4KiB

#[repr(C)]
struct Registers {
    // callee-saved registers
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

    fn new(func: Entry, stack_size: usize) -> Context {
        // allocate stack
        let stack = if stack_size != 0 {
            let mut stack: *mut libc::c_void = ptr::null_mut();
            let result = unsafe {
                libc::posix_memalign(&mut stack as *mut *mut libc::c_void, PAGE_SIZE, stack_size)
            };
            if result != 0 {
                panic!("failed posix_memalign");
            }

            unsafe {
                if libc::mprotect(stack, PAGE_SIZE, libc::PROT_NONE) != 0 {
                    panic!("mprotect");
                }
            }

            stack
        } else {
            ptr::null_mut()
        };

        let regs = Registers {
            d8: 0,
            d9: 0,
            d10: 0,
            d11: 0,
            d12: 0,
            d13: 0,
            d14: 0,
            d15: 0,
            x19: 0,
            x20: 0,
            x21: 0,
            x22: 0,
            x23: 0,
            x24: 0,
            x25: 0,
            x26: 0,
            x27: 0,
            x28: 0,
            x30: entry_point as u64,
            sp: stack as u64 + stack_size as u64,
        };

        Context {
            regs: regs,
            stack: stack,
            entry: func,
        }
    }
}

extern "C" {
    fn set_context(ctx: *mut Registers) -> u64;
    fn switch_context(ctx: *mut Registers) -> !;
}

static mut CTX_MAIN: Option<Box<Context>> = None;
static mut UNUSED_STACK: *mut libc::c_void = ptr::null_mut();
static mut CONTEXTS: LinkedList<Box<Context>> = LinkedList::new();

fn dummy() {
    loop {}
}

pub fn spawn_from_main(func: Entry, stack_size: usize) {
    unsafe {
        if let Some(_) = &CTX_MAIN {
            panic!("spawn_from_main is called again before clean up");
        }

        CTX_MAIN = Some(Box::new(Context::new(dummy, 0)));
        if let Some(ctx) = &mut CTX_MAIN {
            if set_context(ctx.get_regs()) == 0 {
                CONTEXTS.push_back(Box::new(Context::new(func, stack_size)));
                let first = CONTEXTS.front_mut().unwrap();
                switch_context(first.get_regs());
            } else {
                rm_unused_stack();
                CTX_MAIN = None;
            }
        }
    }
}

pub fn spawn(func: Entry, stack_size: usize) -> () {
    unsafe {
        CONTEXTS.push_back(Box::new(Context::new(func, stack_size)));
        schedule();
    }
}

pub fn schedule() {
    unsafe {
        if CONTEXTS.len() == 1 {
            return;
        }

        let mut ctx = CONTEXTS.pop_front().unwrap();
        let regs = ctx.get_regs();
        CONTEXTS.push_back(ctx);
        if set_context(regs) == 0 {
            let next = CONTEXTS.front_mut().unwrap();
            switch_context((**next).get_regs());
        } else {
            rm_unused_stack();
        }
    }
}

extern "C" fn entry_point() {
    unsafe {
        let ctx = CONTEXTS.front().unwrap();
        ((**ctx).entry)();

        let ctx = CONTEXTS.pop_front().unwrap();
        UNUSED_STACK = (*ctx).stack;

        match CONTEXTS.front_mut() {
            Some(c) => {
                switch_context((**c).get_regs());
            }
            None => {
                if let Some(c) = &mut CTX_MAIN {
                    switch_context(c.get_regs());
                }
            }
        };
    }
}

unsafe fn rm_unused_stack() {
    if UNUSED_STACK != ptr::null_mut() {
        if libc::mprotect(UNUSED_STACK, PAGE_SIZE, libc::PROT_READ | libc::PROT_WRITE) != 0 {
            panic!("mprotect");
        }

        libc::free(UNUSED_STACK);
        UNUSED_STACK = ptr::null_mut();
    }
}
