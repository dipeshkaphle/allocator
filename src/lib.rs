mod colors;
mod free_list;
mod header;
mod memory_chunk;
pub mod utils;

use std::mem::size_of;

pub const DEFAULT_COLOR: colors::Color = colors::CAML_BLUE;
pub const DEFAULT_TAG: u8 = 0;

// This should always be power of 2
// Due to this property, we can take mod  by just doing & with (CHUNK_SIZE - 1)
pub const CHUNK_SIZE: usize = 256 * 1024 * 1024;

#[no_mangle]
pub extern "C" fn alloc(sz: std::ffi::c_ulonglong) -> *mut u8 {
    let header_size = size_of::<header::Header>();
    let layout = utils::get_layout(sz as usize + header_size);
    let mem = unsafe { std::alloc::alloc(layout) };
    let data_portion = unsafe { mem.add(header_size) };

    unsafe {
        *(mem as *mut header::Header) = header::Header::new(layout.size(), 0, 0);
    }
    data_portion
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8) {
    let header_size = size_of::<header::Header>();
    unsafe {
        let mem = ptr.sub(header_size);
        let allocation_size = *(mem as *mut header::Header);
        std::alloc::dealloc(mem, utils::get_layout(allocation_size.get_size()));
    }
}

#[cfg(test)]
mod tests {
    use std::alloc::Layout;

    #[test]
    fn f() {
        println!("{:?}", Layout::new::<i128>().align());
        println!("{:?}", crate::utils::get_layout(16).align());
    }
}
