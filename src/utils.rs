use std::{alloc::Layout, env};

use crate::{header::Header, value::Value, word::Wsize};

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

#[cfg(target_pointer_width = "16")]
pub const SHIFT: usize = 1;

#[cfg(target_pointer_width = "32")]
pub const SHIFT: usize = 2;

#[cfg(target_pointer_width = "64")]
pub const SHIFT: usize = 3;

pub fn get_actual_wosz_to_request(mut request_wo_sz: Wsize) -> Wsize {
    // We'll just allocate twice as much as the request, if request >= 1MB, else 1MB
    let min_wosz_expand: Wsize = env::var("MIN_EXPANSION_WORDSIZE")
        .ok()
        .and_then(|x| x.parse::<usize>().ok())
        .map(Wsize::new)
        .unwrap_or(Wsize::new((1024 >> SHIFT) * 1024)); // 1MB = (1024*1024)/WORD_SIZE
                                                        // words

    if request_wo_sz >= min_wosz_expand {
        *request_wo_sz.get_val_mut() <<= 1;
    } else {
        request_wo_sz = min_wosz_expand;
    }
    request_wo_sz
}

#[inline(always)]
pub fn get_layout(mem_size: Wsize) -> std::alloc::Layout {
    let next_pow_of_two = mem_size.to_bytesize().next_power_of_two();

    Layout::from_size_align(next_pow_of_two, ALIGN).unwrap()
}

#[inline(always)]
pub fn get_layout_and_actual_expansion_size(mut request_wo_sz: Wsize) -> (Layout, Wsize) {
    request_wo_sz = get_actual_wosz_to_request(request_wo_sz);

    let layout = get_layout(request_wo_sz);
    (layout, Wsize::from_bytesize(layout.size()))
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
pub fn get_next(cur: &Value) -> &mut Value {
    field_ref_mut(cur, 0)
}

#[inline(always)]
pub fn whsize_wosize(wsz: Wsize) -> Wsize {
    Wsize::new(wsz.get_val() + 1)
}
#[inline(always)]
pub fn wosize_whsize(wsz: Wsize) -> Wsize {
    Wsize::new(wsz.get_val() - 1)
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
        unsafe { &mut *($val.0 as *mut Header).sub(1) }
    };
}

#[macro_export]
macro_rules! val_hp {
    ($hp: expr) => {
        unsafe { Value(($hp as *mut Header).add(1) as usize) }
    };
}

#[inline(always)]
pub fn field_ref_mut(val: &Value, index: isize) -> &mut Value {
    let val_as_mut_value = val.0 as *mut Value;

    let offs = unsafe { val_as_mut_value.offset(index) };

    unsafe { &mut *offs }
}

#[inline(always)]
pub fn field_val(val: Value, index: isize) -> Value {
    let val_as_ptr = val.0 as *mut Value;

    let offs = val_as_ptr.wrapping_offset(index);
    // let offs = unsafe { val_as_ptr.offset(index) };

    Value(offs as usize)
}

#[test]
pub fn field_val_test() {
    let mem = field_val(Value(std::ptr::null_mut() as *mut u8 as usize), 1).0 as *mut u8;
    assert_eq!(field_val(Value(mem as usize), -1), Value(0));
    assert_eq!(
        field_val(Value(std::ptr::null_mut() as *mut u8 as usize), 1),
        Value(8)
    );
}
