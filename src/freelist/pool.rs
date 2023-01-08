use crate::{header::Header, value::Value, word::Wsize};

#[repr(C)]
pub struct Pool {
    pub(in crate::freelist) pool_wo_sz: Wsize,
    pub(in crate::freelist) next: *mut Pool,
    pub(in crate::freelist) filler: Value,
    pub(in crate::freelist) hd: Header,
    pub(in crate::freelist) first_field: Value,
}

impl Pool {
    //
    pub fn get_field_wosz_from_pool_wosz(pool_wo_sz: Wsize) -> Wsize {
        pool_wo_sz - Wsize::from_bytesize(std::mem::size_of::<Pool>()) + Wsize::new(1)
    }
}
