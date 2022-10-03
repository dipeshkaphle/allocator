mod types;
pub mod utils;

use std::mem::size_of;

#[no_mangle]
pub extern "C" fn alloc(sz: std::ffi::c_ulonglong) -> *mut u8 {
    let header_size = size_of::<types::Header>();
    let layout = utils::get_layout(sz as usize + header_size);
    let mem = unsafe { std::alloc::alloc(layout) };
    let data_portion = unsafe { mem.add(header_size) };

    unsafe {
        *(mem as *mut types::Header) = types::Header::new(layout.size(), 0, 0);
    }
    data_portion
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8) {
    let header_size = size_of::<types::Header>();
    unsafe {
        let mem = ptr.sub(header_size);
        let allocation_size = *(mem as *mut types::Header);
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
