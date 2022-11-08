#![allow(clippy::mut_from_ref)]
mod colors;
mod free_list;
mod header;
// mod memory_chunk;
mod utils;
mod value;

use std::mem::size_of;

use colors::CAML_BLUE;
use free_list::{nf_allocate, nf_expand_heap, FreeList};
use header::Header;
use utils::{get_header, get_header_mut, val_field};
use value::{Value, VAL_NULL};

use crate::free_list::traverse_fl;

pub const DEFAULT_COLOR: colors::Color = colors::CAML_BLUE;
pub const DEFAULT_TAG: u8 = 0;

#[no_mangle]
pub extern "C" fn alloc(wo_sz: std::ffi::c_ulonglong) -> *mut u8 {
    let mut mem = nf_allocate(wo_sz as usize);
    if Value(mem as usize) == VAL_NULL {
        // add new block and allocate
        let prev_cnt = FreeList::new().nf_iter().count();
        nf_expand_heap(wo_sz as usize);
        assert_eq!(FreeList::new().nf_iter().count(), prev_cnt + 1);
        if cfg!(debug_assertions) {
            traverse_fl(|v| println!("{:?}", v));
        }
        mem = nf_allocate(wo_sz as usize);
    }
    val_field(Value(mem as usize), 1).0 as *mut u8
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8) {
    let header_size = size_of::<header::Header>();
    unsafe {
        let mut mem = ptr.sub(header_size);
        let header = get_header(&mem).clone();
        *get_header_mut(&mut mem) =
            Header::new(header.get_size(), CAML_BLUE, header.get_tag() as _);
    }
}

#[cfg(test)]
mod tests {

    use crate::alloc;

    #[test]
    fn tests() {
        let alloc_mem = alloc(1024 * 1024);
        assert_ne!(alloc_mem, std::ptr::null_mut());
        // dealloc(alloc_mem);
        // //since it's first fit this should pass
        // assert_eq!(alloc(256 * 1024), alloc_mem);
    }
}
