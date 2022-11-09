use crate::utils::SHIFT;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
#[repr(transparent)]
pub struct Wsize(usize);

impl Wsize {
    #[inline(always)]
    pub fn get_val_mut(&mut self) -> &mut usize {
        &mut self.0
    }
    #[inline(always)]
    pub fn get_val(&self) -> &usize {
        &self.0
    }
    #[inline(always)]
    pub const fn new(val: usize) -> Self {
        Wsize(val)
    }
    #[inline(always)]
    pub fn from_bytesize(bytes: usize) -> Self {
        Wsize(bytes >> SHIFT)
    }
    #[inline(always)]
    pub fn to_bytesize(self) -> usize {
        self.0 << SHIFT
    }
}
