use crate::{
    colors::CAML_BLUE,
    header::Header,
    utils::{self, get_header_mut, get_ptr_at_offset},
    DEFAULT_COLOR, DEFAULT_TAG,
};

#[derive(Clone, Copy)]
pub struct FreeList {
    size: usize,
    start: *mut u8,
    cur_offset: usize,
}

impl FreeList {
    pub fn iter(&self) -> FreeListIter {
        FreeListIter::new(*self)
    }
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

    pub fn get_cur_ptr(&self) -> *mut u8 {
        utils::get_ptr_at_offset(self.start, self.cur_offset)
    }

    pub fn get_next_offset(&self) -> usize {
        let header = unsafe { *(self.get_cur_ptr() as *mut Header) };
        (self.cur_offset + header.get_size()) & (self.size - 1)
    }

    pub fn get_next_ptr(&self) -> *mut u8 {
        utils::get_ptr_at_offset(self.start, self.get_next_offset())
    }

    pub fn find_first(&mut self, sz: usize) -> Option<*mut u8> {
        const MIN_SIZE: usize = 16;
        let it = self
            .iter()
            .find(|e| e.get_header().get_color() == CAML_BLUE && e.get_header().get_size() >= sz);
        match it {
            Some(ptr) => {
                if ptr.get_header().get_size() >= (sz + MIN_SIZE) {
                    // split
                    let (fst, _) = ptr.split(sz);
                    Some(fst.get_ptr())
                } else {
                    Some(ptr.get_ptr())
                }
            }
            None => None,
        }
    }
}

pub struct FreeListIter {
    fl: FreeList,
    visited_start: bool,
}

impl FreeListIter {
    pub fn new(fl: FreeList) -> FreeListIter {
        FreeListIter {
            fl,
            visited_start: false,
        }
    }
}
impl Iterator for FreeListIter {
    type Item = FreeListPtr;
    fn next(&mut self) -> Option<Self::Item> {
        let cur_ptr = self.fl.get_cur_ptr();
        let next_offset = self.fl.get_next_offset();
        if cur_ptr == self.fl.start && self.visited_start {
            None
        } else {
            self.visited_start = true;
            self.fl.cur_offset = next_offset;
            Some(FreeListPtr::new(cur_ptr))
        }
    }
}

pub struct FreeListPtr {
    ptr: *mut u8,
}
impl FreeListPtr {
    pub fn new(ptr: *mut u8) -> FreeListPtr {
        FreeListPtr { ptr }
    }
    pub fn get_header(&self) -> Header {
        *utils::get_header(&self.ptr)
    }
    pub fn get_ptr(self) -> *mut u8 {
        self.ptr
    }
    pub fn split(self, first_half_size: usize) -> (FreeListPtr, FreeListPtr) {
        let hd = self.get_header();
        let mut ptr = self.get_ptr();
        let first_header = Header::new(first_half_size, DEFAULT_COLOR, DEFAULT_TAG);
        let second_header =
            Header::new(hd.get_size() - first_half_size, DEFAULT_COLOR, DEFAULT_TAG);
        let mut next_ptr = get_ptr_at_offset(ptr, first_half_size);
        *get_header_mut(&mut ptr) = first_header;
        *get_header_mut(&mut next_ptr) = second_header;
        (FreeListPtr::new(ptr), FreeListPtr::new(next_ptr))
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
        let mut free_list = FreeList::new(size, mem);
        assert_eq!(free_list.get_next_offset(), 0);
        assert_eq!(free_list.get_next_ptr(), mem);
        assert_eq!(free_list.get_cur_ptr(), mem);

        assert_eq!(free_list.iter().count(), 1);
        assert_eq!(
            free_list.iter().next().unwrap().get_header().get_size(),
            1024
        );

        // would have caused a split
        let ptr = free_list.find_first(16);
        assert_eq!(utils::get_header(&ptr.unwrap()).get_size(), 16);

        // split made it 2
        assert_eq!(free_list.iter().count(), 2);

        // wont cause split as remaining wont be more than MIN_SIZE
        let _ = free_list.find_first(1008);
        assert_eq!(free_list.iter().count(), 2);
    }
}
