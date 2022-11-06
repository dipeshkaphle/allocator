mod colors;
mod free_list;
mod header;
// mod memory_chunk;
mod utils;
mod value;

use std::{mem::size_of, ptr::null_mut};

use colors::CAML_BLUE;
use header::Header;
use utils::{get_header, get_header_mut};

pub const DEFAULT_COLOR: colors::Color = colors::CAML_BLUE;
pub const DEFAULT_TAG: u8 = 0;

// This should always be power of 2
// Due to this property, we can take mod  by just doing & with (CHUNK_SIZE - 1)
pub const CHUNK_SIZE: usize = 256 * 1024 * 1024;

#[no_mangle]
pub extern "C" fn alloc(sz: std::ffi::c_ulonglong) -> *mut u8 {
    // FreeList::new().find_first()
    todo!()
    // let header_size = size_of::<header::Header>();
    // let layout = utils::get_layout(sz as usize + header_size);
    // let mem_chunk_ptr = unsafe { &mut *MemoryChunk::get() };

    // unsafe {
    // mem_chunk_ptr
    // .allocate(layout.size())
    // .map(|x| x.add(header_size))
    // .unwrap_or(null_mut())
    // }
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8) {
    let header_size = size_of::<header::Header>();
    unsafe {
        let mut mem = ptr.sub(header_size);
        let header = get_header(&mem).clone();
        *get_header_mut(&mut mem) =
            Header::new(header.get_size(), CAML_BLUE, header.get_tag() as _);
    }
}

#[cfg(test)]
mod tests {

    use crate::{alloc, dealloc};

    #[test]
    fn tests() {
        let alloc_mem = alloc(256 * 1024);
        assert_ne!(alloc_mem, std::ptr::null_mut());
        dealloc(alloc_mem);
        //since it's first fit this should pass
        assert_eq!(alloc(256 * 1024), alloc_mem);
    }
}
