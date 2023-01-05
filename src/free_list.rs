use std::{alloc::Layout, env, sync::Once};

use crate::{
    colors::CAML_BLUE,
    header::Header,
    utils::{self, field_val, get_header_mut, get_next, val_bp, whsize_wosize, wosize_whsize},
    val_hp,
    value::{Val, Value, VAL_NULL},
    word::Wsize,
    DEFAULT_TAG,
};

pub struct NfAllocator {
    globals: NfGlobals,
    #[cfg(debug_assertions)]
    last_expandheap_start_end: (usize, usize),
    num_of_heap_expansions: usize,
}

impl NfAllocator {
    pub fn new() -> Self {
        let sentinel = Box::leak(Box::new(SentinelType {
            filler1: Value(0),
            h: Header::new(0, CAML_BLUE, 0),
            first_field: VAL_NULL,
            filler2: Value(0),
        }));
        let head = val_bp(std::ptr::addr_of_mut!(sentinel.first_field) as *mut u8);
        Self {
            globals: NfGlobals {
                cur_wsz: Wsize::new(0),
                nf_head: head,
                nf_prev: head,
                nf_last: head,
            },
            #[cfg(debug_assertions)]
            last_expandheap_start_end: (0usize, 0usize),
            num_of_heap_expansions: 0usize,
        }
    }

    #[inline(always)]
    pub fn get_globals(&mut self) -> &mut NfGlobals {
        &mut self.globals
    }
    #[inline(always)]
    #[cfg(debug_assertions)]
    pub fn get_start_end_after_heap_expand(&self) -> (usize, usize) {
        self.last_expandheap_start_end
    }

    #[inline(always)]
    pub fn get_num_of_expansions(&self) -> usize {
        self.num_of_heap_expansions
    }

    fn nf_allocate_block(&mut self, prev: Value, cur: Value, wh_sz: Wsize) -> *mut Header {
        let hd_sz = cur.get_header().get_wosize();

        if *cur.get_header().get_wosize().get_val() < (wh_sz.get_val() + 1) {
            // If we're here, the size of header is exactly wh_sz or wo_sz[=wosize_whsize(wh_sz)]
            // This is only ever called from nf_allocate, we will never be breaking this invariant.
            // So the size of header can only be these two values
            //
            // # equals wo_sz
            //
            // We'll change the header to size zero inside this branch. But later on before
            // returning,it's changed back to wo_sz(the requested size)
            //
            // # equals to wh_sz
            //
            // We'll change the header to have size 0 in this branch. The next header right after
            // it is what we must return. IMP: this must be handled while merging. Any value that
            // we get, it might be succeeding an empty block header,so that check must be made.
            //
            //
            // The reason we're setting the header here is so that we can actually merge it later.
            // If we dont keep track of this header's 0 size, we wont know it's useless later on
            // and it will forever create a gap which wont be merged.
            //

            self.get_globals().cur_wsz -= whsize_wosize(cur.get_header().get_wosize());
            *get_next(&prev) = *get_next(&cur);
            *cur.get_header() = Header::new(0, CAML_BLUE, 0);

            // If the pointer we returned was nf_last, we change nf_last
            // This way we're always keeping track of nf_last properly
            if cur == self.get_globals().nf_last {
                self.get_globals().nf_last = prev;
            }
        } else {
            self.get_globals().cur_wsz -= wh_sz;
            *cur.get_header() = Header::new(
                cur.get_header().get_wosize().get_val() - wh_sz.get_val(),
                CAML_BLUE,
                0,
            );
        }

        // since we always split and return the right half,we must calculate the offset at which we split.
        //
        // case wo_sz == hd_sz => -1, this causes the cur.get_header() to have right size
        //
        // case wh_sz == hd_sz => 0, empty block is already there, it'll put header for block to be
        // returned properly
        //
        // case hd_sz >= wh_sz + 1 => positive value, the split block will have proper header
        let offset = *hd_sz.get_val() as isize - *wh_sz.get_val() as isize;

        // Set the header for the memory that we'll be returning
        let val = field_val(cur, offset + 1);
        *val.get_header() = Header::new(*wosize_whsize(wh_sz).get_val(), CAML_BLUE, 0);

        self.get_globals().nf_prev = prev;

        field_val(cur, offset).0 as *mut Header
    }

    pub fn nf_allocate(&mut self, wo_sz: Wsize) -> *mut Header {
        assert!(*wo_sz.get_val() >= 1);
        let it = FreeList::new(self.get_globals()).find_next(wo_sz);
        match it {
            None => VAL_NULL.0 as *mut Header,
            Some(it) => {
                self.nf_allocate_block(it.get_actual_prev(), it.get_cur(), whsize_wosize(wo_sz))
            }
        }
    }

    // We assume this never fails
    pub fn allocate_for_heap_expansion(request_layout: &Layout) -> Value {
        let no_of_bytes_in_layout = request_layout.size();
        let no_of_words_in_layout = Wsize::from_bytesize(no_of_bytes_in_layout);

        // Assuming this'll never fail
        let mut mem_hd = unsafe { std::alloc::alloc_zeroed(*request_layout) };

        assert_ne!(mem_hd, std::ptr::null_mut());

        *get_header_mut(&mut mem_hd) =
            Header::new(no_of_words_in_layout.get_val() - 1, CAML_BLUE, 0);

        let mem_hd = mem_hd as *mut Header;
        val_hp!(mem_hd)
    }

    pub fn nf_expand_heap(&mut self, request_wo_sz: Wsize) {
        let (layout, _) = utils::get_layout_and_actual_expansion_size(request_wo_sz);

        let no_of_bytes_in_layout = layout.size();

        let memory = Self::allocate_for_heap_expansion(&layout);

        #[cfg(debug_assertions)]
        {
            let hp_as_usize = field_val(memory, -1).0;
            self.last_expandheap_start_end = (hp_as_usize, hp_as_usize + no_of_bytes_in_layout);
        }

        self.num_of_heap_expansions += 1;

        // self.nf_add_block(field_val(mem_hd_val, 1));
        self.nf_add_block(memory)
    }

    fn nf_add_block(&mut self, val: Value) {
        let it = FreeList::new(self.get_globals())
            .nf_iter()
            .find(|e| e.get_cur() > val && e.get_prev() < val);
        self.get_globals().cur_wsz += whsize_wosize(val.get_header().get_wosize());
        match it {
            None => {
                // means its the last most address
                *get_next(&self.get_globals().nf_last) = val;
                self.get_globals().nf_last = val;
                *get_next(&self.get_globals().nf_last) = VAL_NULL;
            }
            Some(it) => {
                *get_next(&val) = it.get_cur();
                *get_next(&it.get_actual_prev()) = val;
            }
        }
    }

    pub fn nf_deallocate(&mut self, val: Value) {
        self.get_globals().cur_wsz += whsize_wosize(val.get_header().get_wosize());
        if val > self.get_globals().nf_last {
            let prev = self.get_globals().nf_last;
            *get_next(&self.get_globals().nf_last) = val;
            self.get_globals().nf_last = val;
            *get_next(&self.get_globals().nf_last) = VAL_NULL;
            #[cfg(not(feature = "no_merge"))]
            if try_merge(prev, val) {
                self.get_globals().nf_last = prev;
                *get_next(&self.get_globals().nf_last) = VAL_NULL;
            }
            return;
        }

        if *get_next(&self.get_globals().nf_head) == VAL_NULL
            || val.0 < get_next(&self.get_globals().nf_head).0
        {
            let prev_first = *get_next(&self.get_globals().nf_head);
            *get_next(&self.get_globals().nf_head) = val;
            *get_next(&val) = prev_first;
            if prev_first != VAL_NULL {
                // We must set nf_last to be val as *get_next( nf_head) == VAL_NULL => the list was
                // empty. Must change nf_last
                self.get_globals().nf_last = val;

                #[cfg(not(feature = "no_merge"))]
                let _ = try_merge(val, prev_first);
            }
            return;
        }

        if let Some(it) = FreeList::new(self.get_globals())
            .nf_iter()
            .find(|it| it.get_cur() > val && it.get_prev() < val)
        {
            *get_next(&val) = it.get_cur();
            *get_next(&it.get_actual_prev()) = val;
            #[cfg(not(feature = "no_merge"))]
            {
                let _ = try_merge(val, it.get_cur());
                let _ = try_merge(it.get_actual_prev(), val);
            }
        }
    }
}

#[repr(C)]
pub struct SentinelType {
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
pub struct NfGlobals {
    cur_wsz: Wsize,
    nf_head: Value,
    nf_prev: Value,
    nf_last: Value,
    // Doing get_next on this nf_head, nf_prev and nf_head should always be valid, this is to be maintained
}

impl NfGlobals {
    #[inline(always)]
    fn get() -> &'static mut Self {
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

#[derive(Debug)]
pub struct FreeList<'a> {
    globals: &'a mut NfGlobals,
}

impl FreeList<'_> {
    pub fn nf_iter(&mut self) -> NfIter<'_> {
        // NfIter::new(self)
        let prev = self.globals.nf_prev;
        NfIter {
            globals: self.globals,
            prev,
            visited_start_once: false,
        }
    }
    pub fn new(g: &mut NfGlobals) -> FreeList {
        FreeList { globals: g }
    }

    fn find_next(&mut self, wo_sz: Wsize) -> Option<NfIterVal> {
        self.nf_iter()
            .find(|e| e.get_cur().get_header().get_wosize().get_val() >= wo_sz.get_val())
    }
}

pub struct NfIter<'a> {
    globals: &'a mut NfGlobals,
    prev: Value,
    visited_start_once: bool,
}

impl NfIter<'_> {
    fn get_globals_mut(&mut self) -> &mut NfGlobals {
        self.globals
    }
    fn get_globals(&self) -> &NfGlobals {
        self.globals
    }
}

#[derive(Debug)]
pub struct NfIterVal {
    prev: Value,
    cur: Value,
    prev_is_sentinel: bool,
}
impl NfIterVal {
    #[inline(always)]
    pub fn get_cur(&self) -> Value {
        self.cur
    }
    #[inline(always)]
    pub fn get_prev(&self) -> Value {
        if !self.prev_is_sentinel {
            return self.prev;
        }
        VAL_NULL
    }

    // This is not public, can only be used within this module
    // We would want to call this when we're changing the next value for prev that is generated by
    // iterator.
    // This is used in NfAllocator::nf_allocate_block ,NfAllocator::nf_add_block and
    // NfAllocator::nf_deallocate
    #[inline(always)]
    fn get_actual_prev(&self) -> Value {
        self.prev
    }
}

impl Iterator for NfIter<'_> {
    type Item = NfIterVal; // (prev, cur)
    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.prev;
        let next = *get_next(&cur);

        if self.prev == self.get_globals().nf_prev && self.visited_start_once {
            None
        } else {
            self.visited_start_once = true;
            if next == VAL_NULL {
                self.get_globals_mut().nf_last = cur;
                self.prev = self.get_globals().nf_head;
                return self.next();
            }
            self.prev = next;
            Some(Self::Item {
                prev: cur,
                cur: next,
                prev_is_sentinel: cur == self.get_globals().nf_head,
            })
        }
    }
}

static mut GLOBAL_ALLOC: NfAllocator = NfAllocator {
    globals: NfGlobals {
        cur_wsz: Wsize::new(0),
        nf_head: Value(0),
        nf_prev: Value(0),
        nf_last: Value(0),
    },
    #[cfg(debug_assertions)]
    last_expandheap_start_end: (0usize, 0usize),
    num_of_heap_expansions: 0usize,
};

pub fn get_global_allocator() -> &'static mut NfAllocator {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        GLOBAL_ALLOC.globals.nf_head = NfGlobals::get().nf_head;
        GLOBAL_ALLOC.globals.nf_prev = NfGlobals::get().nf_prev;
        GLOBAL_ALLOC.globals.nf_last = NfGlobals::get().nf_last;
    });

    unsafe { &mut GLOBAL_ALLOC }
}

fn try_merge(prev: Value, cur: Value) -> bool {
    let prev_wosz = prev.get_header().get_wosize();
    let prev_next_val = field_val(prev, (*prev_wosz.get_val()) as _);
    if prev_next_val == field_val(cur, -1) {
        *get_next(&prev) = *get_next(&cur);
        *prev.get_header() = Header::new(
            *prev_wosz.get_val() + whsize_wosize(cur.get_header().get_wosize()).get_val(),
            CAML_BLUE,
            DEFAULT_TAG,
        );
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        colors::CAML_BLUE,
        header::Header,
        hp_val,
        utils::{self, whsize_wosize},
        val_hp,
        value::Val,
        value::Value,
        word::Wsize,
    };

    use super::{FreeList, NfAllocator};

    #[test]
    fn allocate_for_heap_expansion_test() {
        let request_wo_sz = 1024;
        let layout = utils::get_layout(Wsize::new(request_wo_sz));
        let memory = NfAllocator::allocate_for_heap_expansion(&layout);
        assert_eq!(
            memory.get_header().get_wosize(),
            Wsize::new(request_wo_sz - 1)
        );
        assert_eq!(memory.get_header().get_color(), CAML_BLUE);
        unsafe { std::alloc::dealloc(hp_val!(memory) as *mut Header as *mut u8, layout) };
    }

    #[test]
    fn test() {
        let mut allocator = NfAllocator::new();

        // nothing present in freelist
        assert!(FreeList::new(allocator.get_globals()).nf_iter().count() == 0);

        let intended_expansion_size = Wsize::new(1024 * 1024); // Expand the heap with a chunk of size
                                                               // 1024*1024 words i.e (1024**2) *
                                                               // WORD_SIZE bytes

        let (layout, _actual_expansion_size) =
            utils::get_layout_and_actual_expansion_size(intended_expansion_size);

        let actual_expansion_size = Wsize::from_bytesize(layout.size());

        // nf_expand_heap heap will actually allocate for size=actual_expansion_size instead of
        // intended_expansion_size
        allocator.nf_expand_heap(intended_expansion_size);

        assert_eq!(allocator.get_globals().cur_wsz, actual_expansion_size);

        // 1 chunk is present in freelist after expansion
        assert!(FreeList::new(allocator.get_globals()).nf_iter().count() == 1);

        let mut allocations = vec![
            Some(allocator.nf_allocate(Wsize::new(1024))), // allocates 1024 + 1 word
            Some(allocator.nf_allocate(Wsize::new(1024))), // allocates 1024 + 1 word
        ];

        // initial size -(1024 + 1 word( ret by whsize_wosize) allocated twice)
        let cur_wsz = actual_expansion_size - ((whsize_wosize(Wsize::new(1024))) * 2);

        assert_eq!(allocator.get_globals().cur_wsz, cur_wsz);

        let to_be_freed = allocations.get_mut(0).unwrap().take().unwrap();
        assert!(allocations.get(0).unwrap().is_none());

        let allocatable_memory_left = FreeList::new(allocator.get_globals())
            .nf_iter()
            .fold(Wsize::new(0), |acc, x| {
                acc + x.get_cur().get_header().get_wosize()
            });

        //The following allocation will force the empty block case in nf_allocate_block
        let hp = allocator.nf_allocate(allocatable_memory_left - Wsize::new(1));

        assert_eq!(
            val_hp!(hp).get_header().get_wosize(),
            allocatable_memory_left - Wsize::new(1)
        );
        //Assert the size of empty block that lies 1 word before hp
        assert_eq!(
            Value(hp as usize).get_header().get_wosize(), // treat hp as val, it'll treat empty
            // block as it's header
            Wsize::new(0)
        );
        allocations.push(Some(hp));

        // This must've made the free list empty
        assert_eq!(allocator.get_globals().cur_wsz, Wsize::new(0));
        assert_eq!(
            FreeList::new(allocator.get_globals())
                .nf_iter()
                .fold(Wsize::new(0), |acc, x| {
                    acc + x.get_cur().get_header().get_wosize()
                }),
            Wsize::new(0)
        );

        // Freeing the first allocation
        let to_be_freed_header = val_hp!(to_be_freed).get_header().clone();
        allocator.nf_deallocate(val_hp!(to_be_freed));

        assert_eq!(
            allocator.get_globals().cur_wsz,
            to_be_freed_header.get_wosize() + Wsize::new(1)
        );

        let allocatable_memory_left = to_be_freed_header.get_wosize();

        // Allocating exactly allocatable_memory_left will again empty the freelist
        let hp = allocator.nf_allocate(allocatable_memory_left);

        assert_ne!(hp, std::ptr::null_mut());
        assert_eq!(allocator.get_globals().cur_wsz, Wsize::new(0));
        assert_eq!(
            FreeList::new(allocator.get_globals())
                .nf_iter()
                .fold(Wsize::new(0), |acc, x| {
                    acc + x.get_cur().get_header().get_wosize()
                }),
            Wsize::new(0)
        );
    }
}
