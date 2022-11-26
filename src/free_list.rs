use std::{
    env::{self},
    sync::Once,
};

use crate::{
    colors::CAML_BLUE,
    header::Header,
    utils::{self, field_val, get_header_mut, get_next, val_bp, whsize_wosize, wosize_whsize},
    value::{Val, Value, VAL_NULL},
    word::Wsize,
    DEFAULT_TAG,
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

#[cfg(debug_assertions)]
#[derive(Debug)]
pub struct NfGlobals {
    pub cur_wsz: Wsize,
    pub nf_head: Value,
    pub nf_prev: Value,
    pub nf_last: Value,
}

#[cfg(debug_assertions)]
static mut LAST_EXPANDHEAP_START_END: (usize, usize) = (0, 0);

#[cfg(debug_assertions)]
pub fn get_start_end_after_heap_expand() -> (usize, usize) {
    unsafe { LAST_EXPANDHEAP_START_END }
}

#[cfg(not(debug_assertions))]
#[derive(Debug)]
struct NfGlobals {
    pub cur_wsz: Wsize,
    pub nf_head: Value,
    pub nf_prev: Value,
    pub nf_last: Value,
}

impl NfGlobals {
    #[inline(always)]
    pub fn get() -> &'static mut Self {
        static mut NF_GLOBAL: NfGlobals = NfGlobals {
            cur_wsz: Wsize::new(0),
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

pub fn clear_fl() {
    unsafe {
        SENTINEL.first_field = VAL_NULL;
        *NfGlobals::get() = NfGlobals {
            cur_wsz: Wsize::new(0),
            nf_head: val_bp(std::ptr::addr_of_mut!(SENTINEL.first_field) as *mut u8),
            nf_prev: Value(0),
            nf_last: Value(0),
        };
    }
    NfGlobals::get().nf_last = NfGlobals::get().nf_head;
    NfGlobals::get().nf_prev = NfGlobals::get().nf_head;
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

    fn find_next(&mut self, wo_sz: Wsize) -> Option<NfIterVal> {
        self.nf_iter()
            .find(|e| e.cur.get_header().get_wosize().get_val() >= wo_sz.get_val())
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

fn nf_allocate_block(prev: Value, cur: Value, wh_sz: Wsize) -> *mut Header {
    // #[cfg(debug_assertions)]
    // println!("[nf_allocate_block] prev: {:?}\ncur:{:?}", prev, cur);

    let hd_sz = cur.get_header().get_wosize();
    if *cur.get_header().get_wosize().get_val() < (wh_sz.get_val() + 1) {
        *NfGlobals::get().cur_wsz.get_val_mut() -=
            whsize_wosize(cur.get_header().get_wosize()).get_val();
        *get_next(&prev) = *get_next(&cur);
        *cur.get_header() = Header::new(0, CAML_BLUE, 0);
    } else {
        *NfGlobals::get().cur_wsz.get_val_mut() -= wh_sz.get_val();
        *cur.get_header() = Header::new(
            cur.get_header().get_wosize().get_val() - wh_sz.get_val(),
            CAML_BLUE,
            0,
        );
    }

    // #[cfg(debug_assertions)]
    // println!("[nf_allocate_block] {:?}", cur);

    let offset = *hd_sz.get_val() as isize - *wh_sz.get_val() as isize;

    // Set the header for the memory that we'll be returning
    let vf = field_val(cur, offset + 1);
    *vf.get_header() = Header::new(*wosize_whsize(wh_sz).get_val(), CAML_BLUE, 0);

    NfGlobals::get().nf_prev = prev;

    // #[cfg(debug_assertions)]
    // println!("[nf_allocate_block] prev: {:?}\ncur:{:?}", prev, cur);

    (field_val(cur, offset).0 as *mut usize) as *mut Header
}

pub fn nf_allocate(wo_sz: Wsize) -> *mut Header {
    assert!(*wo_sz.get_val() >= 1);
    let it = FreeList::new().find_next(wo_sz);
    match it {
        None => VAL_NULL.0 as *mut Header,
        Some(it) => nf_allocate_block(it.prev, it.cur, whsize_wosize(wo_sz)),
    }
}

fn get_actual_wosz_to_request(mut request_wo_sz: Wsize) -> Wsize {
    // We'll just allocate twice as much as the request, if request >= 1MB, else 1MB
    let min_wosz_expand: Wsize = env::var("MIN_EXPANSION_WORDSIZE")
        .ok()
        .and_then(|x| x.parse::<usize>().ok())
        .map(Wsize::new)
        .unwrap_or(Wsize::new((1024 >> 3) * 1024 * 1024));

    if request_wo_sz >= min_wosz_expand {
        *request_wo_sz.get_val_mut() <<= 1;
    } else {
        request_wo_sz = min_wosz_expand;
    }
    request_wo_sz
}

pub fn nf_expand_heap(mut request_wo_sz: Wsize) {
    request_wo_sz = get_actual_wosz_to_request(request_wo_sz);
    // alloc expects the request in bytes
    let layout = utils::get_layout(request_wo_sz);

    // Assuming this'll never fail
    let mut mem_hd = unsafe { std::alloc::alloc_zeroed(layout) };

    let mem_hd_val = Value(mem_hd as usize);
    assert_ne!(mem_hd, std::ptr::null_mut());
    *get_header_mut(&mut mem_hd) = Header::new(
        Wsize::from_bytesize(layout.size()).get_val() - 1,
        CAML_BLUE,
        0,
    );

    #[cfg(debug_assertions)]
    unsafe {
        LAST_EXPANDHEAP_START_END = (mem_hd_val.0, mem_hd_val.0 + layout.size());
    }

    // #[cfg(debug_assertions)]
    // println!("[nf_expand_heap]{:?}", field_val(mem_hd_val, 1));

    nf_add_block(field_val(mem_hd_val, 1));
}

fn nf_add_block(val: Value) {
    let it = FreeList::new().nf_iter().find(|e| e.cur > val);
    *NfGlobals::get().cur_wsz.get_val_mut() +=
        whsize_wosize(val.get_header().get_wosize()).get_val();
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
    let prev_wosz = prev.get_header().get_wosize();
    let prev_next_val = field_val(prev, (*prev_wosz.get_val()) as _);
    if prev_next_val == field_val(cur, -1) {
        *get_next(&prev) = *get_next(&cur);
        *prev.get_header() = Header::new(
            *prev_wosz.get_val() + whsize_wosize(cur.get_header().get_wosize()).get_val(),
            CAML_BLUE,
            DEFAULT_TAG,
        )
    }
}

pub fn nf_deallocate(val: Value) {
    *NfGlobals::get().cur_wsz.get_val_mut() +=
        whsize_wosize(val.get_header().get_wosize()).get_val();
    if val > NfGlobals::get().nf_last {
        let prev = NfGlobals::get().nf_last;
        *get_next(&NfGlobals::get().nf_last) = val;
        NfGlobals::get().nf_last = val;
        *get_next(&NfGlobals::get().nf_last) = VAL_NULL;
        #[cfg(not(feature = "no_merge"))]
        try_merge(prev, val);
        return;
    }

    if val.0 <= get_next(&NfGlobals::get().nf_head).0 {
        let prev_first = *get_next(&NfGlobals::get().nf_head);
        *get_next(&NfGlobals::get().nf_head) = val;
        *get_next(&val) = prev_first;
        #[cfg(not(feature = "no_merge"))]
        try_merge(val, prev_first);
        return;
    }

    if let Some(it) = FreeList::new()
        .nf_iter()
        .find(|it| it.cur > val && it.prev < val)
    {
        *get_next(&val) = it.cur;
        *get_next(&it.prev) = val;
        #[cfg(not(feature = "no_merge"))]
        {
            try_merge(val, it.cur);
            try_merge(it.prev, val);
        }
    }
}

pub fn traverse_fl<F>(f: F)
where
    F: Fn(Value),
{
    println!("======Traversing FreeList=========");
    FreeList::new().nf_iter().for_each(|v| f(v.cur));
    println!("====================================");
}
