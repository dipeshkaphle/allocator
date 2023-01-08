use std::alloc::Layout;

use crate::{
    colors::{CAML_BLACK, CAML_BLUE},
    freelist::{fl::FreeList, pool::Pool},
    header::Header,
    utils::{
        self, field_val, get_header_mut, get_next, get_pool_mut, val_bp, whsize_wosize,
        wosize_whsize,
    },
    val_hp,
    value::{Value, VAL_NULL},
    word::Wsize,
    DEFAULT_TAG,
};

use super::globals::{NfGlobals, SentinelType};

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

    #[cfg(feature = "check_invariants")]
    fn check_nf_allocate_block_invariant(&mut self, prev: Value, cur: Value, wh_sz: Wsize) {
        assert!(
            (prev == self.get_globals().nf_head && cur == *get_next(&prev)) || (prev.0 < cur.0),
            "[nf_allocate_block] prev<cur invariant broken"
        );
        assert!(
            whsize_wosize(cur.get_header().get_wosize()) >= wh_sz,
            "The invariant(block size should be enough) to be maintained is broken. to_be_allocated block doesnt have enough size to satisfy the request."
        );
    }

    fn nf_allocate_block(&mut self, prev: Value, cur: Value, wh_sz: Wsize) -> *mut Header {
        let hd_sz = cur.get_header().get_wosize();

        #[cfg(feature = "check_invariants")]
        self.check_nf_allocate_block_invariant(prev, cur, wh_sz);

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
                *get_next(&self.get_globals().nf_last) = VAL_NULL;
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

        // Set the header for the memory that we'll be returning, IMP: Make it have CAML_BLACK color
        let val = field_val(cur, offset + 1);
        *val.get_header() = Header::new(*wosize_whsize(wh_sz).get_val(), CAML_BLACK, 0);

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

        #[cfg(feature = "check_invariants")]
        assert_ne!(
            mem_hd,
            std::ptr::null_mut(),
            "Heap expansion never failing invariant failed"
        );

        let pool = get_pool_mut(&mut mem_hd);
        pool.pool_wo_sz = no_of_words_in_layout;
        pool.next = std::ptr::null_mut();
        pool.hd = Header::new(
            // Should have size = no_of_words_in_layout - sizeof(Pool) + 1 word(considering the first_field field)
            *Pool::get_field_wosz_from_pool_wosz(no_of_words_in_layout).get_val(),
            CAML_BLUE,
            DEFAULT_TAG,
        );

        // field_val(val_bp(mem_hd), 4) would also work, it's guaranteed to be at  4th index since
        // we're using repr(C)
        Value(std::ptr::addr_of_mut!(pool.first_field) as usize)
    }

    pub fn nf_expand_heap(&mut self, request_wo_sz: Wsize) {
        let (layout, _) = utils::get_layout_and_actual_expansion_size(request_wo_sz);

        let no_of_bytes_in_layout = layout.size();

        let memory = Self::allocate_for_heap_expansion(&layout);

        #[cfg(debug_assertions)]
        {
            let hp_as_usize = field_val(memory, -1).0;
            self.last_expandheap_start_end = (
                hp_as_usize,
                hp_as_usize + memory.get_header().get_wosize().to_bytesize(),
            );
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
    #[cfg(feature = "check_invariants")]
    pub fn verify_nf_last_invariant(&mut self) {
        assert!(
            FreeList::new(self.get_globals())
                .nf_iter()
                .all(|it| it.get_prev() < it.get_cur()),
            "Sorted free list invariant broken"
        );

        let largest_cur_val = FreeList::new(self.get_globals())
            .nf_iter()
            .fold(Value(0), |acc, e| Value(acc.0.max(e.get_cur().0)));

        let nf_last = get_global_allocator().get_globals().nf_last;
        let nf_head = get_global_allocator().get_globals().nf_head;
        assert!(
            (nf_last == nf_head) || largest_cur_val == nf_last,
            "NfLast == LargestValueInFreeList Invariant failed.\nNfLast:{nf_last:?}\nLargestInFreeList:{largest_cur_val:?}\n",
        );
    }

    #[cfg(not(feature = "no_merge"))]
    fn merge_and_update_global(&mut self, left: Value, right: Value) {
        let merged = utils::try_merge(left, right);
        if merged {
            if self.get_globals().nf_last == right {
                self.get_globals().nf_last = left;
            }
            if self.get_globals().nf_prev == right {
                self.get_globals().nf_prev = left;
            }
        }
    }

    pub fn nf_deallocate(&mut self, val: Value) {
        self.get_globals().cur_wsz += whsize_wosize(val.get_header().get_wosize());

        // let nf_head = self.get_globals().nf_head;
        // let nf_last = self.get_globals().nf_last;

        *val.get_header() = Header::new(
            *val.get_header().get_wosize().get_val(),
            CAML_BLUE,
            DEFAULT_TAG,
        );

        if val > self.get_globals().nf_last {
            let prev = self.get_globals().nf_last;
            *get_next(&self.get_globals().nf_last) = val;
            self.get_globals().nf_last = val;

            #[cfg(not(feature = "no_merge"))]
            self.merge_and_update_global(prev, val);

            *get_next(&self.get_globals().nf_last) = VAL_NULL;
            return;
        }

        if *get_next(&self.get_globals().nf_head) == VAL_NULL
            || val.0 < get_next(&self.get_globals().nf_head).0
        {
            let prev_first = *get_next(&self.get_globals().nf_head);

            *get_next(&self.get_globals().nf_head) = val;
            *get_next(&val) = prev_first;

            if prev_first != VAL_NULL {
                #[cfg(not(feature = "no_merge"))]
                self.merge_and_update_global(val, prev_first);
            } else {
                // We must set nf_last to be val as *get_next( nf_head) == VAL_NULL => the list was
                // empty. Must change nf_last
                self.get_globals().nf_last = val;
                *get_next(&self.get_globals().nf_last) = VAL_NULL;
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
                self.merge_and_update_global(val, it.get_cur());
                self.merge_and_update_global(it.get_actual_prev(), val);
            }
        } else {
            FreeList::new(self.get_globals()).nf_iter().for_each(|it| {
                eprintln!(
                    "Prev:{:?}\nCur:{:?}\n------------------------------------------",
                    it.get_prev(),
                    it.get_cur()
                );
            });

            // FreeList::new(self.get_globals())
            // .nf_iter()
            // .for_each(|x| eprintln!("{x:?}"));
            panic!(
                " \n\n===> Dellocation Request for: {val:?}\n Globals: {:?}\n",
                self.get_globals(),
            );
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
