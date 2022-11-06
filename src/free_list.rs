use std::sync::Once;

use crate::{
    colors::CAML_BLUE,
    header::Header,
    utils::{field, get_next, val_bp, val_field, whsize_wosize, wosize_whsize},
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
    pub fn new(fl: FreeList) -> NfIter {
        Self {
            prev: NfGlobals::get().nf_prev,
            visited_start_once: false,
        }
    }
}

#[derive(Debug)]
pub struct NfIterVal {
    prev: Value,
    cur: Value,
}

impl Iterator for NfIter {
    type Item = NfIterVal; // (prev, cur)
    fn next(&mut self) -> Option<Self::Item> {
        let mut cur = self.prev;
        let next = *get_next(&mut cur);
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

fn nf_allocate_block(mut prev: Value, mut cur: Value, wh_sz: usize) -> *mut Header {
    // println!("prev: {:?}\ncur:{:?}", prev, cur);

    let hd_sz = cur.get_header().get_size();
    if cur.get_header().get_size() < (wh_sz + 1) {
        NfGlobals::get().cur_wsz -= whsize_wosize(cur.get_header().get_size());
        *get_next(&mut prev) = *get_next(&mut cur);
        *cur.get_header() = Header::new(0, CAML_BLUE, 0);
    } else {
        NfGlobals::get().cur_wsz -= wh_sz;
        *cur.get_header() = Header::new(cur.get_header().get_size() - wh_sz, CAML_BLUE, 0);
    }
    let offset = hd_sz as isize - wh_sz as isize;

    // Set the header for the memory that we'll be returning
    *val_field(cur, offset + 1).get_header() = Header::new(wosize_whsize(wh_sz), CAML_BLUE, 0);

    NfGlobals::get().nf_prev = prev;

    // println!("prev: {:?}\ncur:{:?}", prev, cur);

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

fn nf_add_block(mut val: Value) {
    let it = FreeList::new().nf_iter().find(|e| e.cur > val);
    NfGlobals::get().cur_wsz += whsize_wosize(val.get_header().get_size());
    match it {
        None => {
            // means its the last most address
            *get_next(&mut NfGlobals::get().nf_last) = val;
            NfGlobals::get().nf_last = val;
        }
        Some(mut it) => {
            *get_next(&mut val) = it.cur;
            *get_next(&mut it.prev) = val;
        }
    }
}

#[cfg(test)]
mod freelist_tests {
    use crate::{
        colors::CAML_BLUE,
        free_list::{nf_add_block, nf_allocate, FreeList, NfGlobals},
        header::Header,
        utils::{self, field, get_header_mut, val_field, WORD_SIZE},
        value::{Val, Value},
    };

    #[test]
    fn test() {
        let size = 1024;
        let layout = utils::get_layout(size * WORD_SIZE);
        let mut mem_hd = unsafe { std::alloc::alloc(layout) };
        let mem_hd_val = Value(mem_hd as usize);

        assert_ne!(mem_hd, std::ptr::null_mut());
        *get_header_mut(&mut mem_hd) = Header::new(size - 1, CAML_BLUE, 0);

        assert_eq!(FreeList::new().nf_iter().count(), 0);

        nf_add_block(val_field(mem_hd_val, 1));

        assert_eq!(FreeList::new().nf_iter().count(), 1);

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
