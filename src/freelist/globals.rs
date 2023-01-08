use std::sync::Once;

use crate::{
    colors::CAML_BLUE,
    freelist::pool::Pool,
    header::Header,
    utils::val_bp,
    value::{Value, VAL_NULL},
    word::Wsize,
};

#[repr(C)]
pub struct SentinelType {
    pub(in crate::freelist) filler1: Value,
    pub(in crate::freelist) h: Header,
    pub(in crate::freelist) first_field: Value,
    pub(in crate::freelist) filler2: Value,
}

static mut SENTINEL: SentinelType = SentinelType {
    filler1: Value(0),
    h: Header::new(0, CAML_BLUE, 0),
    first_field: VAL_NULL,
    filler2: Value(0),
};

#[derive(Debug)]
#[repr(C)]
pub struct NfGlobals {
    pub(in crate::freelist) cur_wsz: Wsize,
    pub(in crate::freelist) nf_head: Value,
    pub(in crate::freelist) nf_prev: Value,
    pub(in crate::freelist) nf_last: Value,
    pub(in crate::freelist) pool_head: *mut Pool,
    // Doing get_next on this nf_head, nf_prev and nf_head should always be valid, this is to be maintained
}

impl NfGlobals {
    #[inline(always)]
    pub fn get() -> &'static mut Self {
        static mut FIRST_POOL: Pool = Pool {
            pool_wo_sz: Wsize::new(0),
            prev: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
            filler: Value(0),
            hd: Header::new(0, CAML_BLUE, 0),
            first_field: Value(0),
        };
        static mut NF_GLOBAL: NfGlobals = NfGlobals {
            cur_wsz: Wsize::new(0),
            nf_head: Value(0),
            nf_prev: Value(0),
            nf_last: Value(0),
            pool_head: std::ptr::null_mut(),
        };
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            unsafe {
                // Circular linked list invariant
                FIRST_POOL.next = std::ptr::addr_of_mut!(FIRST_POOL);
                FIRST_POOL.prev = FIRST_POOL.next;

                NF_GLOBAL.nf_head = val_bp(std::ptr::addr_of_mut!(SENTINEL.first_field) as *mut u8);
                NF_GLOBAL.nf_last = NF_GLOBAL.nf_head;
                NF_GLOBAL.nf_prev = NF_GLOBAL.nf_head;
                NF_GLOBAL.pool_head = FIRST_POOL.next;
            };
        });

        unsafe { &mut NF_GLOBAL }
    }
}
