use std::sync::Once;

use crate::{
    colors::CAML_BLUE,
    header::Header,
    utils::{
        self, get_header_mut, get_next, val_bp, val_field, whsize_wosize, wosize_whsize, WORD_SIZE,
    },
    value::{Val, Value, VAL_NULL},
};

#[repr(C)]
struct SentinelType {
    filler1: Value,
    h: Header,
    first_field: Value,
    filler2: Value,
}

static mut SENTINEL: SentinelType = SentinelType {
    filler1: Value(0),
    h: Header::new(0, CAML_BLUE, 0),
    first_field: VAL_NULL,
    filler2: Value(0),
};

#[derive(Debug)]
struct NfGlobals {
    pub cur_wsz: usize,
    pub nf_head: Value,
    pub nf_prev: Value,
    pub nf_last: Value,
}

impl NfGlobals {
    #[inline(always)]
    pub fn get() -> &'static mut Self {
        static mut NF_GLOBAL: NfGlobals = NfGlobals {
            cur_wsz: 0,
            nf_head: Value(0),
            nf_prev: Value(0),
            nf_last: Value(0),
        };

        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            unsafe {
                NF_GLOBAL.nf_head = val_bp(std::ptr::addr_of_mut!(SENTINEL.first_field) as *mut u8);
                NF_GLOBAL.nf_last = NF_GLOBAL.nf_head;
                NF_GLOBAL.nf_prev = NF_GLOBAL.nf_head;
            };
        });

        unsafe { &mut NF_GLOBAL }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FreeList {}

impl FreeList {
    pub fn nf_iter(&mut self) -> NfIter {
        NfIter::new(*self)
    }
    pub fn new() -> FreeList {
        FreeList {}
    }

    fn find_next(&mut self, wo_sz: usize) -> Option<NfIterVal> {
        self.nf_iter()
            .find(|e| e.cur.get_header().get_size() >= wo_sz)
    }
}

pub struct NfIter {
    prev: Value,
    visited_start_once: bool,
}

impl NfIter {
    pub fn new(_fl: FreeList) -> NfIter {
        Self {
            prev: NfGlobals::get().nf_prev,
            visited_start_once: false,
        }
    }
}

#[derive(Debug)]
pub struct NfIterVal {
    pub prev: Value,
    pub cur: Value,
}

impl Iterator for NfIter {
    type Item = NfIterVal; // (prev, cur)
    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.prev;
        let next = *get_next(&cur);
        if self.prev == NfGlobals::get().nf_prev && self.visited_start_once {
            None
        } else {
            self.visited_start_once = true;
            if next == VAL_NULL {
                NfGlobals::get().nf_last = cur;
                self.prev = NfGlobals::get().nf_head;
                // cur = NfGlobals::get().nf_head;
                // next = get_next(&mut cur);
                return self.next();
            }
            self.prev = next;
            Some(Self::Item {
                prev: cur,
                cur: next,
            })
        }
    }
}

fn nf_allocate_block(prev: Value, cur: Value, wh_sz: usize) -> *mut Header {
    if cfg!(debug_assertions) {
        println!("[nf_allocate_block] prev: {:?}\ncur:{:?}", prev, cur);
    }

    let hd_sz = cur.get_header().get_size();
    if cur.get_header().get_size() < (wh_sz + 1) {
        NfGlobals::get().cur_wsz -= whsize_wosize(cur.get_header().get_size());
        *get_next(&prev) = *get_next(&cur);
        *cur.get_header() = Header::new(0, CAML_BLUE, 0);
    } else {
        NfGlobals::get().cur_wsz -= wh_sz;
        *cur.get_header() = Header::new(cur.get_header().get_size() - wh_sz, CAML_BLUE, 0);
    }
    if cfg!(debug_assertions) {
        println!("[nf_allocate_block] {:?}", cur);
    }

    let offset = hd_sz as isize - wh_sz as isize;

    // Set the header for the memory that we'll be returning
    let vf = val_field(cur, offset + 1);
    *vf.get_header() = Header::new(wosize_whsize(wh_sz), CAML_BLUE, 0);

    NfGlobals::get().nf_prev = prev;

    if cfg!(debug_assertions) {
        println!("[nf_allocate_block] prev: {:?}\ncur:{:?}", prev, cur);
    }

    (val_field(cur, offset).0 as *mut usize) as *mut Header
}

pub fn nf_allocate(wo_sz: usize) -> *mut Header {
    assert!(wo_sz >= 1);
    let it = FreeList::new().find_next(wo_sz);
    match it {
        None => VAL_NULL.0 as *mut Header,
        Some(it) => nf_allocate_block(it.prev, it.cur, whsize_wosize(wo_sz)),
    }
}

pub fn nf_expand_heap(mut request_wo_sz: usize) {
    // We'll just allocate twice as much as the request, if request >= 1MB, else 1MB
    const MIN_WOSZ_EXPAND: usize = 1024 * 1024;
    if request_wo_sz >= MIN_WOSZ_EXPAND {
        request_wo_sz <<= 1;
    } else {
        request_wo_sz = MIN_WOSZ_EXPAND;
    }

    // alloc expects the request in bytes
    let layout = utils::get_layout(request_wo_sz * WORD_SIZE);

    // Assuming this'll never fail
    let mut mem_hd = unsafe { std::alloc::alloc_zeroed(layout) };
    let mem_hd_val = Value(mem_hd as usize);
    assert_ne!(mem_hd, std::ptr::null_mut());
    *get_header_mut(&mut mem_hd) = Header::new((layout.size() >> 3) - 1, CAML_BLUE, 0);

    if cfg!(debug_assertions) {
        println!("[nf_expand_heap]{:?}", val_field(mem_hd_val, 1));
    }
    nf_add_block(val_field(mem_hd_val, 1));
}

fn nf_add_block(val: Value) {
    let it = FreeList::new().nf_iter().find(|e| e.cur > val);
    NfGlobals::get().cur_wsz += whsize_wosize(val.get_header().get_size());
    match it {
        None => {
            // means its the last most address
            *get_next(&NfGlobals::get().nf_last) = val;
            NfGlobals::get().nf_last = val;
            *get_next(&NfGlobals::get().nf_last) = VAL_NULL;
        }
        Some(it) => {
            *get_next(&val) = it.cur;
            *get_next(&it.prev) = val;
        }
    }
}

fn try_merge(prev: Value, cur: Value) {
    // no-op right now
}

pub fn nf_deallocate(val: Value) {
    NfGlobals::get().cur_wsz += whsize_wosize(val.get_header().get_size());
    if val > NfGlobals::get().nf_last {
        let prev = NfGlobals::get().nf_last;
        *get_next(&NfGlobals::get().nf_last) = val;
        NfGlobals::get().nf_last = val;
        *get_next(&NfGlobals::get().nf_last) = VAL_NULL;
        try_merge(prev, val);
        return;
    }

    let it = FreeList::new()
        .nf_iter()
        .find(|it| it.cur > val && it.prev < val)
        .unwrap();
    *get_next(&val) = it.cur;
    *get_next(&it.prev) = val;
    try_merge(val, it.cur);
    try_merge(it.prev, val);
}

pub fn traverse_fl<F>(f: F)
where
    F: Fn(Value),
{
    println!("======Traversing FreeList=========");
    FreeList::new().nf_iter().for_each(|v| f(v.cur));
    println!("====================================");
}

#[cfg(test)]
mod freelist_tests {
    use crate::{
        colors::CAML_BLUE,
        free_list::{nf_add_block, nf_allocate, FreeList, NfGlobals, VAL_NULL},
        header::Header,
        utils::{self, get_header_mut, get_next, val_field, WORD_SIZE},
        value::{Val, Value},
    };

    #[test]
    fn test() {
        assert_eq!(*get_next(&NfGlobals::get().nf_last), VAL_NULL);

        let size = 1024;
        let layout = utils::get_layout(size * WORD_SIZE);
        let mut mem_hd = unsafe { std::alloc::alloc(layout) };
        let mem_hd_val = Value(mem_hd as usize);

        assert_ne!(mem_hd, std::ptr::null_mut());
        *get_header_mut(&mut mem_hd) = Header::new(size - 1, CAML_BLUE, 0);

        assert_eq!(FreeList::new().nf_iter().count(), 0);

        nf_add_block(val_field(mem_hd_val, 1));

        assert_eq!(FreeList::new().nf_iter().count(), 1);
        assert_eq!(*get_next(&NfGlobals::get().nf_last), VAL_NULL);

        assert_eq!(
            FreeList::new()
                .nf_iter()
                .next()
                .unwrap()
                .cur
                .get_header()
                .get_size(),
            size - 1
        );

        // should cause a split
        let small_sz = 16;
        let ptr = nf_allocate(small_sz);
        assert_eq!(unsafe { (*ptr).get_size() }, small_sz);

        let rem_size = size - 1 - (small_sz + 1);
        assert_eq!(
            unsafe { (*nf_allocate(size - 1 - (small_sz + 1))).get_size() },
            rem_size
        );
    }
}
