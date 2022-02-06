#[no_mangle]
pub extern "C" fn c_init_sdk() {
    crate::init_sdk();
}
