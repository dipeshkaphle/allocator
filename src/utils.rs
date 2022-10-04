use std::alloc::Layout;

use crate::header::Header;

#[cfg(target_pointer_width = "16")]
static ALIGN: usize = 2usize;

#[cfg(target_pointer_width = "32")]
static ALIGN: usize = 4usize;

#[cfg(target_pointer_width = "64")]
static ALIGN: usize = 8usize;

pub fn get_layout(mem_size: usize) -> std::alloc::Layout {
    let next_pow_of_two = mem_size.next_power_of_two();

    // let _align = POW_OF_TWO_ARR.binary_search(&next_pow_of_two).unwrap();

    Layout::from_size_align(next_pow_of_two, ALIGN).unwrap()
}

pub fn get_ptr_at_offset(start: *mut u8, offset: usize) -> *mut u8 {
    unsafe { start.add(offset) }
}

pub fn get_header(ptr: &*mut u8) -> &Header {
    unsafe { &*(*ptr as *mut Header) }
}

pub fn get_header_mut(ptr: &mut *mut u8) -> &mut Header {
    unsafe { &mut *(*ptr as *mut Header) }
}
