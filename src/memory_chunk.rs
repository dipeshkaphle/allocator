use std::sync::Once;

use crate::{free_list::FreeList, utils, CHUNK_SIZE};

// static mut MEMORY: *mut MemoryChunk = std::ptr::null_mut();
static mut MEMORY: Option<*mut MemoryChunk> = None;

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
mod memory_chunk_tests {
    use super::MemoryChunk;

    #[test]
    fn get_test() {
        assert_ne!(MemoryChunk::get(), std::ptr::null_mut());

        // We're using a singleton, so we should get same address each time
        let mem = MemoryChunk::get();
        assert_eq!(MemoryChunk::get(), mem);
    }
}
