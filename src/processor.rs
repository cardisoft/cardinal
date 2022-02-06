//! Platform independent fs event processor.
use crate::consts::DB_PATH;
use crate::database::{Database, PartialDatabase};
use crate::fsevent::EventId;
use crate::{fs_entry::DiskEntry, fsevent::FsEvent};
use crate::{runtime, utils};

use anyhow::{bail, Context, Result};
use crossbeam::channel::{self, Receiver, Sender, TryRecvError, TrySendError};
use fsevent_sys::FSEventStreamEventId;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use tracing::info;

use std::path::Path;
use std::{collections::BTreeSet, path::PathBuf};

pub static PROCESSOR: OnceCell<Processor> = OnceCell::new();

pub struct Processor {
    /// Raw fs events receiver channel from system.
    events_receiver: Receiver<FsEvent>,

    /// Bounded fs events FIFO pipe for displaying.
    limited_fs_events: (Sender<FsEvent>, Receiver<FsEvent>),
    /// The event id all the events begins with.
    // TODO(ldm0) is this really needed?
    event_id: EventId,
    /// Paths of current file system.
    core_paths: Mutex<BTreeSet<PathBuf>>,
}

impl Processor {
    const FS_EVENTS_CHANNEL_LEN: usize = 1024;
    pub fn new(event_id: EventId, events_receiver: Receiver<FsEvent>) -> Self {
        let (sender, receiver) = channel::bounded(Self::FS_EVENTS_CHANNEL_LEN);
        Self {
            limited_fs_events: (sender, receiver),
            events_receiver,
            event_id,
            core_paths: Mutex::new(BTreeSet::new()),
        }
    }

    /// Non blocking move fs_event in. If filled, it will drop oldest fs event
    /// repeatedly until a fs_event is pushed.
    fn fill_fs_event(&self, event: FsEvent) -> Result<()> {
        let mut event = Some(event);
        loop {
            match self.limited_fs_events.0.try_send(event.take().unwrap()) {
                Ok(()) => break,
                Err(TrySendError::Disconnected(_)) => bail!("fs events channel closed!"),
                Err(TrySendError::Full(give_back)) => {
                    match self.limited_fs_events.1.try_recv() {
                        Ok(x) => drop(x),
                        Err(TryRecvError::Disconnected) => bail!("fs events channel disconnected"),
                        Err(TryRecvError::Empty) => {}
                    };
                    event = Some(give_back);
                }
            }
        }
        Ok(())
    }

    /// Take out fs_event cache of current processor.
    fn take_fs_events(&self) -> Vec<FsEvent> {
        // Due to non atomic channel recv, double the size of possible receiving vec.
        let max_take_num = 2 * self.limited_fs_events.0.len();
        let mut fs_events = Vec::with_capacity(max_take_num);
        while let Ok(event) = self.limited_fs_events.1.try_recv() {
            if fs_events.len() >= max_take_num {
                break;
            }
            fs_events.push(event);
        }
        fs_events
    }

    /// Non-blocking process a event.
    pub fn process_event(&self) -> Result<()> {
        let event = self
            .events_receiver
            .recv()
            .context("System events channel closed.")?;
        self.core_paths.lock().insert(event.path.clone());
        // Provide raw fs event.
        self.fill_fs_event(event).context("fill fs event failed.")?;
        Ok(())
    }

    pub fn get_db(&self) -> Result<Database> {
        let db = match Database::from_fs(Path::new(DB_PATH)) {
            Ok(x) => x,
            Err(e) => {
                info!(?e, "Get db failed, try scanning.");
                let mut partial_db = PartialDatabase::scan_fs();
                info!("Fs scanning completes.");
                while let Ok(event) = self.events_receiver.try_recv() {
                    partial_db.merge();
                }
                partial_db.complete_merge(utils::current_timestamp())
            }
        };
        Ok(db)
    }

    pub fn block_on(&self) -> Result<()> {
        let mut db = self.get_db().context("Get db failed.")?;
        loop {
            if let Err(e) = self.process_event() {
                panic!("processor is down. {}", e);
            }
        }
    }
}

/// Get raw fs events from global processor. Capacity is limited due to the
/// memory pressure, so only the first few(currently 1024) events will be provided.
pub fn take_fs_events() -> Vec<FsEvent> {
    PROCESSOR
        .get()
        .map(|x| x.take_fs_events())
        .unwrap_or_default()
}

fn on_db_completes() {}
