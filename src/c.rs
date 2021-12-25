#[no_mangle]
pub extern "C" fn c_init_sdk() {
    crate::init_sdk();
}

#[no_mangle]
pub unsafe extern "C" fn c_get_events(
    context: *const i8,
    callback: Option<unsafe extern "C" fn(*const i8, *const i8)>,
) {
    let events = crate::processor::take_fs_events();
    let bytes = crate::fsevent::write_events_to_bytes(&events);
    callback.map(|c| unsafe { c(context, bytes.as_ptr() as _) });
}
