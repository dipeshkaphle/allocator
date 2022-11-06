use std::alloc::Layout;

use crate::{header::Header, value::Value};

#[cfg(target_pointer_width = "16")]
static ALIGN: usize = 2usize;

#[cfg(target_pointer_width = "32")]
static ALIGN: usize = 4usize;

#[cfg(target_pointer_width = "64")]
static ALIGN: usize = 8usize;

#[cfg(target_pointer_width = "16")]
pub const WORD_SIZE: usize = 2usize;

#[cfg(target_pointer_width = "32")]
pub const WORD_SIZE: usize = 4usize;

#[cfg(target_pointer_width = "64")]
pub const WORD_SIZE: usize = 8usize;

#[inline(always)]
pub fn get_layout(mem_size: usize) -> std::alloc::Layout {
    let next_pow_of_two = mem_size.next_power_of_two();

    Layout::from_size_align(next_pow_of_two, ALIGN).unwrap()
}

#[inline(always)]
pub fn get_ptr_at_offset(start: *mut u8, offset: usize) -> *mut u8 {
    unsafe { start.add(offset) }
}

#[inline(always)]
pub fn get_header(ptr: &*mut u8) -> &Header {
    unsafe { &*(*ptr as *mut Header) }
}

#[inline(always)]
pub fn get_header_mut(ptr: &mut *mut u8) -> &mut Header {
    unsafe { &mut *(*ptr as *mut Header) }
}

#[inline(always)]
pub fn get_next(cur: &mut Value) -> &mut Value {
    field(cur, 0)
}

#[inline(always)]
pub fn whsize_wosize(wsz: usize) -> usize {
    wsz + 1
}
#[inline(always)]
pub fn wosize_whsize(wsz: usize) -> usize {
    wsz - 1
}

#[macro_export]
macro_rules! bp_val {
    ($v: expr) => {
        ($v.0 as *mut Value) as *mut u8
    };
}
pub fn val_bp(p: *mut u8) -> Value {
    unsafe { std::mem::transmute(p) }
}

#[macro_export]
macro_rules! hd_bp {
    ($ptr:expr) => {
        unsafe { &mut *($ptr as *mut Header) }
    };
}

#[macro_export]
macro_rules! hp_val {
    ($val: expr) => {
        unsafe { &mut *(val as *mut Header).sub(1) }
    };
}

#[inline(always)]
pub fn field(val: &mut Value, index: isize) -> &mut Value {
    let val_as_mut_value = val.0 as *mut Value;

    let offs = unsafe { val_as_mut_value.offset(index) };

    let _unused = Value(offs as usize);

    unsafe { &mut *offs }
}

#[inline(always)]
pub fn val_field(val: Value, index: isize) -> Value {
    let val_as_ptr = val.0 as *mut Value;

    let offs = unsafe { val_as_ptr.offset(index) };

    let _unused = Value(offs as usize);

    Value(offs as usize)
}
