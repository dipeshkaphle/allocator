use std::ops::{Add, Sub, Mul, SubAssign, AddAssign, MulAssign};

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

impl Mul<usize>  for Wsize{
    type Output = Wsize;
    fn mul(self, rhs: usize) -> Self::Output {
        Wsize(self.0 * rhs)
    }
}
impl MulAssign<usize>  for Wsize{
    fn mul_assign(&mut self, rhs: usize) {
        self.0 *= rhs;
    }
}

impl Add for Wsize {
    type Output = Wsize;
    fn add(self, rhs: Self) -> Self::Output {
        Wsize(self.0 + rhs.0)
    }
}
impl AddAssign for Wsize{
    fn add_assign(&mut self, rhs: Self) {
        self.0+=rhs.0;
    }
}

impl SubAssign for Wsize{
    fn sub_assign(&mut self, rhs: Self) {
        self.0-=rhs.0;
    }
}

impl Sub for Wsize{
    type Output = Wsize;
    fn sub(self, rhs: Self) -> Self::Output {
        Wsize(self.0 - rhs.0)
    }
}

