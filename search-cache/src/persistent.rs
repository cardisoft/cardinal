use crate::{SlabIndex, SlabNode, ThinSlab, name_index::SortedSlabIndices};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    thread::available_parallelism,
    time::Instant,
};
use tracing::info;
use typed_num::Num;

const LSF_VERSION: i64 = 5;

#[derive(Serialize, Deserialize)]
pub struct PersistentStorage {
    pub version: Num<LSF_VERSION>,
    /// The last event id of the cache.
    pub last_event_id: u64,
    /// Root file path of the cache
    pub path: PathBuf,
    /// Ignore paths
    pub ignore_paths: Vec<PathBuf>,
    /// Root index of the slab
    pub slab_root: SlabIndex,
    pub slab: ThinSlab<SlabNode>,
    pub name_index: BTreeMap<Box<str>, SortedSlabIndices>,
    /// The number of rescans emitted before this snapshot.
    pub rescan_count: u64,
}

pub fn read_cache_from_file(path: &Path) -> Result<PersistentStorage> {
    let cache_decode_time = Instant::now();
    let mut bytes = vec![0u8; 4 * 1024];
    let input = File::open(path).context("Failed to open cache file")?;
    let input = zstd::Decoder::new(input).context("Failed to create zstd decoder")?;
    let mut input = BufReader::new(input);
    let storage: PersistentStorage = postcard::from_io((&mut input, &mut bytes))
        .context("Failed to decode cache, maybe the cache is corrupted")?
        .0;
    info!("Cache decode time: {:?}", cache_decode_time.elapsed());
    Ok(storage)
}

pub fn write_cache_to_file(path: &Path, storage: &PersistentStorage) -> Result<()> {
    let cache_encode_time = Instant::now();
    let _ = fs::create_dir_all(path.parent().unwrap());
    let tmp_path = &path.with_extension(".sctmp");
    {
        let output = File::create(tmp_path).context("Failed to create cache file")?;
        let mut output = zstd::Encoder::new(output, 6).context("Failed to create zstd encoder")?;
        output
            .multithread(available_parallelism().map(|x| x.get() as u32).unwrap_or(4))
            .context("Failed to create parallel zstd encoder")?;
        let output = output.auto_finish();
        let mut output = BufWriter::new(output);
        postcard::to_io(storage, &mut output).context("Failed to encode cache")?;
    }
    fs::rename(tmp_path, path).context("Failed to rename cache file")?;
    info!("Cache encode time: {:?}", cache_encode_time.elapsed());
    info!(
        "Cache size: {} MB",
        fs::symlink_metadata(path)
            .context("Failed to get cache file metadata")?
            .len() as f32
            / 1024.
            / 1024.
    );
    Ok(())
}
