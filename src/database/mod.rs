use super::fs_entry::DiskEntry;

use anyhow::{Context, Result};
use bincode::{Decode, Encode};
use std::fs::File;
use std::io::BufWriter;
use std::{io::BufReader, path::Path};
use tracing::{info, instrument};

/// The overall database of Cardinal.
///
/// It's created or loaded on app starts, stored into disk on app closes.
#[derive(Decode, Encode)]
pub struct Database {
    /// The snapshot time of this file system tree.
    time: i64,
    /// Snapshot of the file system tree.
    fs_entry: DiskEntry,
}

impl Database {
    pub fn from_fs(path: &Path) -> Result<Self> {
        let file = File::open(path).context("load db from disk failed.")?;
        let mut file = BufReader::new(file);
        let database = bincode::decode_from_std_read(&mut file, bincode::config::standard())
            .context("Decode failed.")?;
        Ok(database)
    }

    pub fn into_fs(&self, path: &Path) -> Result<()> {
        let file = File::create(path).context("open db file from disk failed.")?;
        let mut file = BufWriter::new(file);
        bincode::encode_into_std_write(self, &mut file, bincode::config::standard())
            .context("Encode failed.")?;
        Ok(())
    }

    pub fn merge(&mut self) {
        self.fs_entry.merge()
    }
}

/// The PartialDatabase contains the file system snapshot and the time starting
/// to take the snapshot.
///
/// To make it really useful, merge the filesystem change(from start time to
/// current time) into the file system.
pub struct PartialDatabase {
    /// The time starting to scan this file system tree.
    create_time: i64,
    /// Snapshot of the file system tree.
    fs_entry: DiskEntry,
}

impl PartialDatabase {
    /// Scan the hierarchy from file system.
    pub fn scan_fs() -> Self {
        let create_time = crate::utils::current_timestamp();
        info!(create_time, "The create time of fs scanning");
        let fs_entry = DiskEntry::from_fs(Path::new("/"));
        Self {
            create_time,
            fs_entry,
        }
    }

    pub fn merge(&mut self) {
        self.fs_entry.merge()
    }

    /// Complete modification merging. Convert self into a serializable database.
    /// `time` is the time of last_fs_event.
    pub fn complete_merge(self, time: i64) -> Database {
        info!(
            create_time = self.create_time,
            merge_complete_time = time,
            "Merging fs events into scanned result completes"
        );
        Database {
            time,
            fs_entry: self.fs_entry,
        }
    }
}
