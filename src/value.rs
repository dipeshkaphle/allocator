use std::fmt::{self, Debug};

use crate::{bp_val, hd_bp, header::Header, utils::val_field};

pub trait Val {
    fn get_header(&self) -> &mut Header;
    fn get_bp(&self) -> *mut u8;
}

pub const VAL_NULL: Value = Value(0);

#[derive(PartialEq, Eq, PartialOrd, Clone, Copy)]
#[repr(transparent)]
pub struct Value(pub usize);

impl Val for Value {
    fn get_header(&self) -> &mut Header {
        let f = val_field(*self, -1);
        let bp = Value(bp_val!(f) as usize);
        hd_bp!(bp.0 as *mut u8)
    }

    fn get_bp(&self) -> *mut u8 {
        bp_val!(*self)
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Value")
            .field("val", &self.0)
            .field("header", &self.get_header())
            .finish()
    }
}