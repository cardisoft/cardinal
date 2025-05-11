mod cache;
mod cli;
mod persistent;
mod query;

use anyhow::{Context, Result};
use bincode::{Decode, Encode};
use cache::SearchCache;
use cardinal_sdk::{
    fsevent::{EventStream, FsEvent},
    fsevent_sys::FSEventStreamEventId,
    utils::current_event_id,
};
use clap::Parser;
use cli::Cli;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use serde::{Deserialize, Serialize};
use std::{fs::Metadata, io::Write, time::UNIX_EPOCH};

#[derive(Serialize, Deserialize, Encode, Decode)]
struct SlabNode {
    parent: Option<usize>,
    children: Vec<usize>,
    name: String,
}

pub struct SlabNodeData {
    pub name: String,
    pub ctime: Option<u64>,
    pub mtime: Option<u64>,
}

impl SlabNodeData {
    pub fn new(name: String, metadata: &Option<Metadata>) -> Self {
        let (ctime, mtime) = match metadata {
            Some(metadata) => ctime_mtime_from_metadata(metadata),
            None => (None, None),
        };
        Self { name, ctime, mtime }
    }
}

fn ctime_mtime_from_metadata(metadata: &Metadata) -> (Option<u64>, Option<u64>) {
    // TODO(ldm0): is this fast enough?
    let ctime = metadata
        .created()
        .ok()
        .and_then(|x| x.duration_since(UNIX_EPOCH).ok())
        .map(|x| x.as_secs());
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|x| x.duration_since(UNIX_EPOCH).ok())
        .map(|x| x.as_secs());
    (ctime, mtime)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let last_event_id = current_event_id();
    let mut cache = if cli.refresh {
        println!("Walking filesystem...");
        SearchCache::walk_fs()
    } else {
        println!("Try reading cache...");
        SearchCache::try_read_persistent_cache().unwrap_or_else(|e| {
            println!("Failed to read cache: {e:?}. Re-walking filesystem...");
            SearchCache::walk_fs()
        })
    };

    let (finish_tx, finish_rx) = bounded::<Sender<SearchCache>>(1);
    let (search_tx, search_rx) = unbounded::<String>();
    let (search_result_tx, search_result_rx) = unbounded::<Result<Vec<String>>>();

    std::thread::spawn(move || {
        let event_stream = spawn_event_watcher("/".to_string(), last_event_id);
        println!("Processing changes during processing");
        loop {
            crossbeam_channel::select! {
                recv(finish_rx) -> tx => {
                    let tx = tx.expect("finish_tx is closed");
                    tx.send(cache).expect("finish_tx is closed");
                    break;
                }
                recv(search_rx) -> query => {
                    let query = query.expect("search_tx is closed");
                    let files = cache.query_files(query);
                    search_result_tx
                        .send(files)
                        .expect("search_result_tx is closed");
                }
                recv(event_stream) -> events => {
                    let events = events.expect("event_stream is closed");
                    cache.handle_fs_events(events);
                }
            }
        }
        println!("fsevent processing is done");
    });

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    loop {
        print!("> ");
        stdout.flush().unwrap();
        let mut line = String::new();
        stdin.read_line(&mut line).unwrap();
        let line = line.trim();
        if line.is_empty() {
            continue;
        } else if line == "/bye" {
            break;
        }

        search_tx
            .send(line.to_string())
            .context("search_tx is closed")?;
        let search_result = search_result_rx
            .recv()
            .context("search_result_rx is closed")?;
        match search_result {
            Ok(path_set) => {
                for (i, path) in path_set.into_iter().enumerate() {
                    println!("[{i}] {path}");
                }
            }
            Err(e) => {
                eprintln!("Failed to search: {e:?}");
            }
        }
    }

    let (cache_tx, cache_rx) = bounded::<SearchCache>(1);
    finish_tx.send(cache_tx).context("cache_tx is closed")?;
    let cache = cache_rx.recv().context("cache_tx is closed")?;
    cache
        .flush_to_file()
        .context("Failed to write cache to file")?;

    Ok(())
}

fn spawn_event_watcher(
    path: String,
    since_event_id: FSEventStreamEventId,
) -> Receiver<Vec<FsEvent>> {
    let (sender, receiver) = unbounded();
    std::thread::spawn(move || {
        EventStream::new(
            &[&path],
            since_event_id,
            0.1,
            Box::new(move |events| {
                let _ = sender.send(events);
            }),
        )
        .block_on()
        .unwrap();
    });
    receiver
}

// TODO(ldm0):
// - file removal routine
// - file addition routine
// - segment search cache(same search routine will be triggered while user is typing, should cache exact[..], suffix, suffix/exact[..])
// [] tui?
// - lazy metadata design
// 或许最后可以在首次扫描过程中就把中间结果 在索引逻辑和搜索逻辑之间抛来抛去，做到边索引边搜索
