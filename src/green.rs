use nix::sys::mman::{mprotect, ProtFlags};
use std::alloc::{alloc, dealloc, Layout};
use std::collections::{HashMap, LinkedList};
use std::ffi::c_void;
use std::ptr;

static mut ID: u64 = 0;

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

struct Context {
    regs: Registers,
    stack: *mut u8,
    stack_layout: Layout,
    entry: Entry,
    id: u64,
}

impl Registers {
    fn new(sp: u64) -> Registers {
        Registers {
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
            sp: sp,
        }
    }
}

impl Context {
    fn get_regs_mut(&mut self) -> *mut Registers {
        &mut self.regs as *mut Registers
    }

    fn get_regs(&self) -> *const Registers {
        &self.regs as *const Registers
    }

    fn new(func: Entry, stack_size: usize, id: u64) -> Context {
        // allocate stack
        let layout = Layout::from_size_align(stack_size, PAGE_SIZE).unwrap();
        let stack = unsafe { alloc(layout) };

        unsafe { mprotect(stack as *mut c_void, PAGE_SIZE, ProtFlags::PROT_NONE).unwrap() };

        let regs = Registers::new(stack as u64 + stack_size as u64);
        Context {
            regs: regs,
            stack: stack,
            stack_layout: layout,
            entry: func,
            id: id,
        }
    }
}

extern "C" {
    fn set_context(ctx: *mut Registers) -> u64;
    fn switch_context(ctx: *const Registers) -> !;
}

struct MappedList<T> {
    map: HashMap<u64, LinkedList<T>>,
}

impl<T> MappedList<T> {
    fn new() -> MappedList<T> {
        MappedList {
            map: HashMap::new(),
        }
    }

    fn push_back(&mut self, key: u64, val: T) {
        if let Some(list) = self.map.get_mut(&key) {
            list.push_back(val);
        } else {
            let mut list = LinkedList::new();
            list.push_back(val);
            self.map.insert(key, list);
        }
    }

    fn pop_front(&mut self, key: u64) -> Option<T> {
        if let Some(list) = self.map.get_mut(&key) {
            let val = list.pop_front();
            if list.len() == 0 {
                self.map.remove(&key);
            }
            val
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.map.clear();
    }
}

static mut CTX_MAIN: Option<Box<Registers>> = None;
static mut UNUSED_STACK: (*mut u8, Layout) = (ptr::null_mut(), Layout::new::<u8>());
static mut CONTEXTS: LinkedList<Box<Context>> = LinkedList::new();
static mut MESSAGES: *mut MappedList<u64> = ptr::null_mut();
static mut WAITING: *mut MappedList<Box<Context>> = ptr::null_mut();

pub fn spawn_from_main(func: Entry, stack_size: usize) {
    unsafe {
        if let Some(_) = &CTX_MAIN {
            panic!("spawn_from_main is called twice");
        }

        CTX_MAIN = Some(Box::new(Registers::new(0)));
        if let Some(ctx) = &mut CTX_MAIN {
            // set up global variables
            let mut msgs = MappedList::new();
            MESSAGES = &mut msgs as *mut MappedList<u64>;

            let mut waiting = MappedList::new();
            WAITING = &mut waiting as *mut MappedList<Box<Context>>;

            if set_context(&mut **ctx as *mut Registers) == 0 {
                CONTEXTS.push_back(Box::new(Context::new(func, stack_size, ID)));
                let first = CONTEXTS.front().unwrap();
                switch_context(first.get_regs());
            }

            rm_unused_stack();

            // clear global variables
            CTX_MAIN = None;
            CONTEXTS.clear();
            MESSAGES = ptr::null_mut();
            WAITING = ptr::null_mut();

            msgs.clear();
            waiting.clear();
        }
    }
}

pub fn spawn(func: Entry, stack_size: usize) -> u64 {
    unsafe {
        ID += 1;
        CONTEXTS.push_back(Box::new(Context::new(func, stack_size, ID)));
        let id = ID;
        schedule();
        id
    }
}

pub fn send(key: u64, msg: u64) {
    unsafe {
        (*MESSAGES).push_back(key, msg);

        if let Some(ctx) = (*WAITING).pop_front(key) {
            CONTEXTS.push_back(ctx);
        }
    }
    schedule();
}

pub fn recv() -> Option<u64> {
    unsafe {
        let key = CONTEXTS.front().unwrap().id;
        println!("key = {}", key);

        // return if a message is aleady sent
        if let Some(msg) = (*MESSAGES).pop_front(key) {
            return Some(msg);
        }

        // panic if there is no other thread
        if CONTEXTS.len() == 1 {
            panic!("waiting never deliverd messages");
        }

        // make the current context waiting
        let mut ctx = CONTEXTS.pop_front().unwrap();
        let regs = ctx.get_regs_mut();
        (*WAITING).push_back(key, ctx);

        // wait context switch
        if set_context(regs) == 0 {
            let next = CONTEXTS.front().unwrap();
            switch_context((**next).get_regs());
        }

        rm_unused_stack();

        // take a value
        (*MESSAGES).pop_front(key)
    }
}

pub fn schedule() {
    unsafe {
        if CONTEXTS.len() == 1 {
            return;
        }

        let mut ctx = CONTEXTS.pop_front().unwrap();
        let regs = ctx.get_regs_mut();
        CONTEXTS.push_back(ctx);
        if set_context(regs) == 0 {
            let next = CONTEXTS.front().unwrap();
            switch_context((**next).get_regs());
        }
        rm_unused_stack();
    }
}

extern "C" fn entry_point() {
    unsafe {
        let ctx = CONTEXTS.front().unwrap();
        ((**ctx).entry)();

        let ctx = CONTEXTS.pop_front().unwrap();
        UNUSED_STACK = ((*ctx).stack, (*ctx).stack_layout);

        match CONTEXTS.front() {
            Some(c) => {
                switch_context((**c).get_regs());
            }
            None => {
                if let Some(c) = &CTX_MAIN {
                    switch_context(&**c as *const Registers);
                }
            }
        };
    }
}

unsafe fn rm_unused_stack() {
    if UNUSED_STACK.0 != ptr::null_mut() {
        mprotect(
            UNUSED_STACK.0 as *mut c_void,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
        )
        .unwrap();
        dealloc(UNUSED_STACK.0, UNUSED_STACK.1);
        UNUSED_STACK = (ptr::null_mut(), Layout::new::<u8>());
    }
}
