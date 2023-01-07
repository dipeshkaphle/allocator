use std::fmt::{self, Debug};

use crate::{
    bp_val,
    colors::CAML_BLUE,
    hd_bp,
    header::Header,
    utils::{field_val, get_next},
};

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
        #[cfg(debug_assertions)]
        assert_ne!(*self, VAL_NULL, "Value is null, can't get header");

        let f = field_val(*self, -1);
        let bp = Value(bp_val!(f) as usize);
        hd_bp!(bp.0 as *mut u8)
    }

    fn get_bp(&self) -> *mut u8 {
        bp_val!(*self)
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == VAL_NULL {
            f.debug_struct("Value").field("val", &"null").finish()
        } else {
            f.debug_struct("Value")
                .field("val", &self.0)
                .field("next", &{
                    if self.get_header().get_color() == CAML_BLUE {
                        format!("{:?}", get_next(self).0)
                    } else {
                        "[NA]This Value is NotFree".to_string()
                    }
                })
                .field("header", &self.get_header())
                .finish()
        }
    }
}
