#![allow(clippy::mut_from_ref)]
mod colors;
mod free_list;
mod header;
mod utils;
mod value;
mod word;

use free_list::{nf_allocate, nf_deallocate, nf_expand_heap, FreeList};
use utils::field_val;
use value::{Value, VAL_NULL};
use word::Wsize;

#[cfg(debug_assertions)]
use crate::free_list::get_start_end_after_heap_expand;

pub const DEFAULT_COLOR: colors::Color = colors::CAML_BLUE;
pub const DEFAULT_TAG: u8 = 0;

#[cfg(debug_assertions)]
static mut MEM_RANGES: Vec<(usize, usize)> = vec![];

#[no_mangle]
pub extern "C" fn alloc(wo_sz: std::ffi::c_ulonglong) -> *mut u8 {
    let mut mem = nf_allocate(Wsize::new(wo_sz as usize));
    static mut EXPANDED_HEAP_CNT: usize = 0;

    #[cfg(debug_assertions)]
    if Value(mem as usize) != VAL_NULL {
        let mem_as_usize = mem as usize;
        if !unsafe { &MEM_RANGES }
            .iter()
            .any(|r| r.0 <= mem_as_usize && mem_as_usize <= r.1)
        {
            panic!(
                "Invalid Memory, Got mem address: {}\n Valid memory addresses: {:?}",
                mem_as_usize,
                unsafe { &MEM_RANGES }
            );
        }
    }

    if cfg!(feature = "no_expand_heap") && unsafe { EXPANDED_HEAP_CNT } == 1 {
        let ret_val = field_val(Value(mem as usize), 1).0 as *mut u8;
        return ret_val;
    }

    if Value(mem as usize) == VAL_NULL {
        // add new block and allocate
        let prev_cnt = FreeList::new().nf_iter().count();
        nf_expand_heap(Wsize::new(wo_sz as usize));

        if cfg!(feature = "no_expand_heap") {
            unsafe {
                EXPANDED_HEAP_CNT += 1;
            }
        }

        #[cfg(debug_assertions)]
        unsafe {
            MEM_RANGES.push(get_start_end_after_heap_expand());
        }

        assert_eq!(FreeList::new().nf_iter().count(), prev_cnt + 1);
        mem = nf_allocate(Wsize::new(wo_sz as usize));

        #[cfg(debug_assertions)]
        if Value(mem as usize) != VAL_NULL {
            let mem_as_usize = mem as usize;
            if !unsafe { &MEM_RANGES }
                .iter()
                .any(|r| r.0 <= mem_as_usize && mem_as_usize <= r.1)
            {
                panic!(
                    "Invalid Memory, Got mem address: {}\n Valid memory addresses: {:?}",
                    mem_as_usize,
                    unsafe { &MEM_RANGES }
                );
            }
        }
    }
    field_val(Value(mem as usize), 1).0 as *mut u8
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8) {
    let val_ptr = Value(ptr as usize);
    if val_ptr == VAL_NULL {
        return;
    }

    #[cfg(debug_assertions)]
    {
        let ptr_as_usize = ptr as usize;
        if !unsafe { &MEM_RANGES }
            .iter()
            .any(|r| r.0 <= ptr_as_usize && ptr_as_usize <= r.1)
        {
            panic!(
                "Invalid Memory, Got mem address: {}\n Valid memory addresses: {:?}",
                ptr_as_usize,
                unsafe { &MEM_RANGES }
            );
        }
    }
    nf_deallocate(val_ptr);
}

#[cfg(test)]
mod tests {

    use crate::{
        alloc, dealloc, free_list::FreeList, utils::whsize_wosize, value::Val, word::Wsize,
    };

    #[test]
    fn tests() {
        let req: usize = 1024 * 8;
        let alloc_mem = alloc(req as u64);
        assert_ne!(alloc_mem, std::ptr::null_mut());
        // traverse_fl(|v| println!("{:?}", v));
        assert_eq!(FreeList::new().nf_iter().count(), 1);

        let total_sz: usize = FreeList::new()
            .nf_iter()
            .map(|v| *whsize_wosize(Wsize::new(v.cur.get_header().get_size())).get_val())
            .sum();

        dealloc(alloc_mem);
        assert_eq!(FreeList::new().nf_iter().count(), 2);

        assert_eq!(
            FreeList::new()
                .nf_iter()
                .map(|v| *whsize_wosize(Wsize::new(v.cur.get_header().get_size())).get_val())
                .sum::<usize>(),
            total_sz + req + 1
        );

        // traverse_fl(|v| println!("{:?}", v));
        // //since it's first fit this should pass
        // assert_eq!(alloc(256 * 1024), alloc_mem);
    }
}
