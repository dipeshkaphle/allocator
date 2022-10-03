use std::sync::Once;

use crate::utils;

/// All possible colors are:
/// - Colors::CAML_WHITE
/// - Colors::CAML_GRAY
/// - Colors::CAML_BLUE
/// - Colors::CAML_BLACK
pub type Color = usize;

mod Colors {
    use super::Color;

    pub const CAML_WHITE: Color = 0usize << 8;
    pub const CAML_GRAY: Color = 1usize << 8;
    pub const CAML_BLUE: Color = 2usize << 8;
    pub const CAML_BLACK: Color = 3usize << 8;
}

pub const DEFAULT_COLOR: Color = Colors::CAML_WHITE;
pub const DEFAULT_TAG: u8 = 0;

// static mut MEMORY: *mut MemoryChunk = std::ptr::null_mut();
static mut MEMORY: Option<*mut MemoryChunk> = None;

// This should always be power of 2
// Due to this property, we can take mod  by just doing & with (CHUNK_SIZE - 1)
pub const CHUNK_SIZE: usize = 256 * 1024 * 1024;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Header(usize);

impl Header {
    #[inline(always)]
    pub fn new(size: usize, color: Color, tag: u8) -> Header {
        Header((size << 10) + color + (tag as usize))
    }
    #[inline(always)]
    pub fn get_tag(&self) -> usize {
        self.0 & 0xff
    }
    #[inline(always)]
    pub fn get_color(&self) -> Color {
        self.0 & 0b1100000000
    }
    #[inline(always)]
    pub fn get_size(&self) -> usize {
        return self.0 >> 10;
        // todo!()
    }
}

pub struct FreeList {
    pub size: usize,
    pub start: *mut u8,
    pub cur_offset: usize,
}

impl FreeList {
    pub fn new(size: usize, ptr: *mut u8) -> FreeList {
        // We'll use this property to take fast mod so that we can have circular counter
        assert!(size.is_power_of_two());

        unsafe {
            *(ptr as *mut Header) = Header::new(size, DEFAULT_COLOR, DEFAULT_TAG);
        }
        FreeList {
            size,
            start: ptr,
            cur_offset: 0,
        }
        //
    }

    #[inline(always)]
    fn get_ptr_at_offset(start: *mut u8, offset: usize) -> *mut u8 {
        unsafe { start.add(offset) }
    }

    #[inline(always)]
    fn get_cur_ptr(&self) -> *mut u8 {
        FreeList::get_ptr_at_offset(self.start, self.cur_offset)
    }

    #[inline(always)]
    fn get_next_offset(&self) -> usize {
        let header = unsafe { *(self.get_cur_ptr() as *mut Header) };
        (self.cur_offset + header.get_size()) & (self.size - 1)
    }

    #[inline(always)]
    fn get_next_ptr(&self) -> *mut u8 {
        FreeList::get_ptr_at_offset(self.start, self.get_next_offset())
    }

    pub fn find_first(&mut self, sz: usize) -> Option<*mut u8> {
        //
        todo!()
    }
}

pub struct MemoryChunk {
    free_list: FreeList,
    next: Option<*mut MemoryChunk>,
}

impl MemoryChunk {
    pub fn get() -> *mut MemoryChunk {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let layout = utils::get_layout(CHUNK_SIZE);
            let mem = unsafe { std::alloc::alloc(layout) };
            unsafe {
                MEMORY = Some(Box::leak(Box::new(MemoryChunk {
                    free_list: FreeList::new(CHUNK_SIZE, mem),
                    next: None,
                })));
            }
        });
        unsafe { MEMORY.unwrap() }
    }

    fn allocate(&mut self) -> Option<*mut u8> {
        todo!()
    }
}

#[cfg(test)]
mod header_tests {

    use crate::types::Colors::CAML_BLUE;

    use super::Header;

    #[test]
    fn test() {
        let hd = Header::new(10, CAML_BLUE, 255);
        assert_eq!(hd.get_size(), 10);
        assert_eq!(hd.get_color(), CAML_BLUE);
        assert_eq!(hd.get_tag(), 255);
    }
}

#[cfg(test)]
mod freelist_tests {
    use crate::utils;

    use super::FreeList;

    #[test]
    fn test() {
        let size = 1024;
        let layout = utils::get_layout(size);
        let mem = unsafe { std::alloc::alloc(layout) };
        let free_list = FreeList::new(size, mem);
        assert_eq!(free_list.get_next_offset(), 0);
        assert_eq!(free_list.get_next_ptr(), mem);
        assert_eq!(free_list.get_cur_ptr(), mem);
    }
}
