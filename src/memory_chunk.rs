use std::sync::Once;

use crate::{
    colors::{CAML_BLACK, CAML_BLUE},
    free_list::{FreeList, FreeListPtr},
    header::Header,
    utils::{self, get_header_mut},
    CHUNK_SIZE,
};

// static mut MEMORY: *mut MemoryChunk = std::ptr::null_mut();
static mut MEMORY: Option<*mut MemoryChunk> = None;

#[derive(Debug)]
pub struct MemoryChunk {
    free_list: FreeList,
    next: Option<*mut MemoryChunk>,
}

impl MemoryChunk {
    fn _get_mock() -> *mut MemoryChunk {
        let layout = utils::get_layout(CHUNK_SIZE);
        let mem = unsafe { std::alloc::alloc(layout) };
        Box::leak(Box::new(MemoryChunk {
            free_list: FreeList::new(CHUNK_SIZE, mem),
            next: None,
        }))
    }
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
    pub fn add_new_chunk(&mut self) {
        let layout = utils::get_layout(CHUNK_SIZE);
        let mem = unsafe { std::alloc::alloc(layout) };
        let next_chunk: *mut MemoryChunk = Box::leak(Box::new(MemoryChunk {
            free_list: FreeList::new(CHUNK_SIZE, mem),
            next: None,
        }));
        let mut cur = self;
        let mut next = cur.next;
        while next.is_some() {
            cur = unsafe { &mut *next.unwrap() };
            next = cur.next;
        }
        cur.next = Some(next_chunk);
    }

    fn allocate_ff(&mut self, sz: usize) -> Option<FreeListPtr> {
        let mut cur_chunk = self;

        loop {
            let ptr = cur_chunk.free_list.iter().find(|e| {
                e.get_header().get_color() == CAML_BLUE && e.get_header().get_size() >= sz
            });
            match ptr {
                None => {
                    if let Some(next) = cur_chunk.next {
                        cur_chunk = unsafe { &mut *next };
                        continue;
                    } else {
                        return None;
                    }
                }
                _ => return ptr,
            }
        }
    }
    pub fn allocate(&mut self, sz: usize) -> Option<*mut u8> {
        const MIN_SIZE: usize = 16;
        let it = self.allocate_ff(sz);
        match it {
            Some(ptr) => {
                if ptr.get_header().get_size() >= (sz + MIN_SIZE) {
                    // split
                    let (fst, _) = ptr.split(sz);
                    let hd = fst.get_header();
                    *get_header_mut(&mut fst.get_ptr()) =
                        Header::new(hd.get_size(), CAML_BLACK, hd.get_tag() as u8);
                    Some(fst.get_ptr())
                } else {
                    let hd = ptr.get_header();
                    *get_header_mut(&mut ptr.get_ptr()) =
                        Header::new(hd.get_size(), CAML_BLACK, hd.get_tag() as u8);
                    Some(ptr.get_ptr())
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod memory_chunk_tests {
    use crate::CHUNK_SIZE;

    use super::MemoryChunk;

    #[test]
    fn get_test() {
        unsafe {
            let _ = Box::from_raw(MemoryChunk::_get_mock());
        }
        let mem = MemoryChunk::_get_mock();
        assert_ne!(mem, std::ptr::null_mut());

        let mem = unsafe { &mut *mem };
        assert_eq!(mem.free_list.iter().count(), 1);
        let _ = mem.allocate(16);
        assert_eq!(mem.free_list.iter().count(), 2);
        let _ = mem.allocate(16);
        assert_eq!(mem.free_list.iter().count(), 3);
        assert!(mem.allocate(CHUNK_SIZE).is_none());
        unsafe {
            let _ = Box::from_raw(mem as *mut MemoryChunk);
        }
    }

    #[test]
    fn add_new_chunk_test() {
        let mem = unsafe { &mut *MemoryChunk::_get_mock() };
        assert!(mem.next.is_none());
        mem.add_new_chunk();
        assert!(mem.next.is_some());
        let _ = unsafe { Box::from_raw(mem.next.unwrap()) };
        let _ = unsafe { Box::from_raw(mem as *mut MemoryChunk) };
    }
}
