use std::fmt::{self, Debug};

use crate::{
    colors::{Color, CAML_BLACK, CAML_BLUE, CAML_GRAY, CAML_WHITE},
    word::Wsize,
};

#[repr(transparent)]
#[derive(Clone)]
pub struct Header(usize);

impl Header {
    pub const fn new(size: usize, color: Color, tag: u8) -> Header {
        Header((size << 10) + color + (tag as usize))
    }
    pub fn get_tag(&self) -> u8 {
        (self.0 & 0xff) as u8
    }
    pub fn get_color(&self) -> Color {
        self.0 & 0b1100000000
    }
    pub fn get_wosize(&self) -> Wsize {
        Wsize::new(self.0 >> 10)
    }
}

impl Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Header")
            .field("size", &self.get_wosize())
            .field("color", {
                match self.get_color() {
                    CAML_BLUE => &"Blue",
                    CAML_GRAY => &"Gray",
                    CAML_BLACK => &"Black",
                    CAML_WHITE => &"White",
                    _ => &"Unknown",
                }
            })
            .field("tag", &self.get_tag())
            .finish()
    }
}

#[cfg(test)]
mod header_tests {

    use crate::colors::CAML_BLUE;

    use super::Header;

    #[test]
    fn test() {
        let hd = Header::new(10, CAML_BLUE, 255);
        assert_eq!(*hd.get_wosize().get_val(), 10);
        assert_eq!(hd.get_color(), CAML_BLUE);
        assert_eq!(hd.get_tag(), 255);
    }
}
