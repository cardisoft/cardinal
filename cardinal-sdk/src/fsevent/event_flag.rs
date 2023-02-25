#![allow(non_upper_case_globals)]
use bitflags::bitflags;
bitflags! {
    pub struct MacEventFlag: u32 {
        const kFSEventStreamEventFlagNone = fsevent_sys::kFSEventStreamEventFlagNone;
        const kFSEventStreamEventFlagMustScanSubDirs = fsevent_sys::kFSEventStreamEventFlagMustScanSubDirs;
        const kFSEventStreamEventFlagUserDropped = fsevent_sys::kFSEventStreamEventFlagUserDropped;
        const kFSEventStreamEventFlagKernelDropped = fsevent_sys::kFSEventStreamEventFlagKernelDropped;
        const kFSEventStreamEventFlagEventIdsWrapped = fsevent_sys::kFSEventStreamEventFlagEventIdsWrapped;
        const kFSEventStreamEventFlagHistoryDone = fsevent_sys::kFSEventStreamEventFlagHistoryDone;
        const kFSEventStreamEventFlagRootChanged = fsevent_sys::kFSEventStreamEventFlagRootChanged;
        const kFSEventStreamEventFlagMount = fsevent_sys::kFSEventStreamEventFlagMount;
        const kFSEventStreamEventFlagUnmount = fsevent_sys::kFSEventStreamEventFlagUnmount;
        const kFSEventStreamEventFlagItemCreated = fsevent_sys::kFSEventStreamEventFlagItemCreated;
        const kFSEventStreamEventFlagItemRemoved = fsevent_sys::kFSEventStreamEventFlagItemRemoved;
        const kFSEventStreamEventFlagItemInodeMetaMod = fsevent_sys::kFSEventStreamEventFlagItemInodeMetaMod;
        const kFSEventStreamEventFlagItemRenamed = fsevent_sys::kFSEventStreamEventFlagItemRenamed;
        const kFSEventStreamEventFlagItemModified = fsevent_sys::kFSEventStreamEventFlagItemModified;
        const kFSEventStreamEventFlagItemFinderInfoMod = fsevent_sys::kFSEventStreamEventFlagItemFinderInfoMod;
        const kFSEventStreamEventFlagItemChangeOwner = fsevent_sys::kFSEventStreamEventFlagItemChangeOwner;
        const kFSEventStreamEventFlagItemXattrMod = fsevent_sys::kFSEventStreamEventFlagItemXattrMod;
        const kFSEventStreamEventFlagItemIsFile = fsevent_sys::kFSEventStreamEventFlagItemIsFile;
        const kFSEventStreamEventFlagItemIsDir = fsevent_sys::kFSEventStreamEventFlagItemIsDir;
        const kFSEventStreamEventFlagItemIsSymlink = fsevent_sys::kFSEventStreamEventFlagItemIsSymlink;
        const kFSEventStreamEventFlagOwnEvent = fsevent_sys::kFSEventStreamEventFlagOwnEvent;
        const kFSEventStreamEventFlagItemIsHardlink = fsevent_sys::kFSEventStreamEventFlagItemIsHardlink;
        const kFSEventStreamEventFlagItemIsLastHardlink = fsevent_sys::kFSEventStreamEventFlagItemIsLastHardlink;
        const kFSEventStreamEventFlagItemCloned = fsevent_sys::kFSEventStreamEventFlagItemCloned;
    }
}

pub enum EventType {
    Unknown,
    File,
    Dir,
    Symlink,
    Hardlink,
}

pub enum ScanType {
    SingleNode,
    Folder,
    /// Something wrong happened, do re-indexing.
    ReScan,
    /// Do nothing, since event id is always updated.
    Nop,
}

impl MacEventFlag {
    pub fn event_type(&self) -> EventType {
        if self.contains(MacEventFlag::kFSEventStreamEventFlagItemIsHardlink)
            | self.contains(MacEventFlag::kFSEventStreamEventFlagItemIsLastHardlink)
        {
            EventType::Hardlink
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemIsSymlink) {
            EventType::Symlink
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemIsDir) {
            EventType::Dir
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemIsFile) {
            EventType::File
        } else {
            EventType::Unknown
        }
    }

    pub fn scan_type(&self) -> ScanType {
        let event_type = self.event_type();
        let is_dir = matches!(event_type, EventType::Dir);
        if self.contains(MacEventFlag::kFSEventStreamEventFlagMustScanSubDirs)
            | self.contains(MacEventFlag::kFSEventStreamEventFlagUserDropped)
            | self.contains(MacEventFlag::kFSEventStreamEventFlagKernelDropped)
        {
            ScanType::ReScan
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagEventIdsWrapped)
            | self.contains(MacEventFlag::kFSEventStreamEventFlagHistoryDone)
        {
            ScanType::Nop
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagRootChanged) {
            // Should never happen since we are watching "/"
            assert!(false);
            ScanType::ReScan
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagUnmount)
            | self.contains(MacEventFlag::kFSEventStreamEventFlagMount)
        {
            assert!(is_dir);
            ScanType::Folder
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemCreated) {
            // creating dir is also single node
            ScanType::SingleNode
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemRemoved) {
            if is_dir {
                ScanType::Folder
            } else {
                ScanType::SingleNode
            }
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemInodeMetaMod) {
            // creating dir is also single node
            ScanType::SingleNode
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemRenamed) {
            if is_dir {
                ScanType::Folder
            } else {
                ScanType::SingleNode
            }
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemModified) {
            assert!(!is_dir);
            ScanType::SingleNode
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemFinderInfoMod)
            | self.contains(MacEventFlag::kFSEventStreamEventFlagItemChangeOwner)
            | self.contains(MacEventFlag::kFSEventStreamEventFlagItemXattrMod)
        {
            // creating dir is also single node
            ScanType::SingleNode
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagOwnEvent) {
            unreachable!()
        } else if self.contains(MacEventFlag::kFSEventStreamEventFlagItemCloned) {
            if is_dir {
                ScanType::Folder
            } else {
                ScanType::SingleNode
            }
        } else {
            panic!("unexpected event: {:?}", self)
        }
    }
}

/// Abstract action of a file system event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventFlag {
    Create,
    Delete,
    Modify,
}

impl TryFrom<MacEventFlag> for EventFlag {
    type Error = MacEventFlag;
    fn try_from(f: MacEventFlag) -> Result<Self, MacEventFlag> {
        if f.contains(MacEventFlag::kFSEventStreamEventFlagItemCreated) {
            Ok(EventFlag::Create)
        } else if f.contains(MacEventFlag::kFSEventStreamEventFlagItemRemoved)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagUnmount)
        {
            Ok(EventFlag::Delete)
        } else if f.contains(MacEventFlag::kFSEventStreamEventFlagItemInodeMetaMod)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagItemXattrMod)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagItemChangeOwner)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagItemFinderInfoMod)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagItemModified)
            // Nowhere to distinguish it's 'from' or 'to'.
            | f.contains(MacEventFlag::kFSEventStreamEventFlagItemRenamed)
            // Nowhere to distinguish it's 'from' or 'to'.
            | f.contains(MacEventFlag::kFSEventStreamEventFlagItemCloned)
        {
            Ok(EventFlag::Modify)
        } else if f.contains(MacEventFlag::kFSEventStreamEventFlagMustScanSubDirs)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagUserDropped)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagKernelDropped)
            | f.contains(MacEventFlag::kFSEventStreamEventFlagEventIdsWrapped)
            // check the FSEvents.h it's implementation will be special
            | f.contains(MacEventFlag::kFSEventStreamEventFlagMount)
        {
            Err(f)
        } else if
        // we are watching root, so this will never happen.
        f.contains(MacEventFlag::kFSEventStreamEventFlagRootChanged)
            // MarkSelf is not set on monitoring
            | f.contains(MacEventFlag::kFSEventStreamEventFlagOwnEvent)
        {
            unreachable!()
        } else {
            Err(f)
        }
    }
}
