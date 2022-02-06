use super::fsevent_flags::MacEventFlag;

use fsevent_sys::FSEventStreamEventId;

use std::{
    ffi::{CStr, OsStr},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

/// Abstract action of a file system event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventFlag {
    Create,
    Delete,
    Modify,
}

impl TryFrom<MacEventFlag> for EventFlag {
    type Error = ();
    fn try_from(f: MacEventFlag) -> Result<Self, ()> {
        if f.contains(MacEventFlag::kFSEventStreamEventFlagItemCreated) {
            Ok(EventFlag::Create)
        } else if f.contains(MacEventFlag::kFSEventStreamEventFlagItemRemoved) {
            Ok(EventFlag::Delete)
        } else if f.contains(MacEventFlag::kFSEventStreamEventFlagItemInodeMetaMod) {
            Ok(EventFlag::Modify)
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
pub struct FsEvent {
    pub path: PathBuf,
    pub flag: EventFlag,
    pub id: FSEventStreamEventId,
}

impl FsEvent {
    pub(crate) fn from_raw(path: *const i8, flag: u32, id: u64) -> Self {
        let path = unsafe { CStr::from_ptr(path) };
        let path = OsStr::from_bytes(path.to_bytes());
        let path = PathBuf::from(path);
        let flag = MacEventFlag::from_bits_truncate(flag);
        let flag = flag
            .try_into()
            .expect("convert mac event flag to abstract event flag failed.");
        FsEvent { path, flag, id }
    }
}
