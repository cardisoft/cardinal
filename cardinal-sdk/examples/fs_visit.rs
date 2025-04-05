use bincode::{Decode, Encode};
use cardinal_sdk::name_pool::NamePool;
use fswalk::{Node, WalkData, walk_it};
use mimalloc::MiMalloc;
use serde::{Deserialize, Serialize};
use slab::Slab;
use std::{
    collections::BTreeMap,
    fs::{self, File, Metadata},
    io::BufWriter,
    path::PathBuf,
    time::{Instant, UNIX_EPOCH},
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Serialize, Deserialize, Encode, Decode)]
struct SlabNode {
    parent: Option<usize>,
    children: Vec<usize>,
    name: String,
}

impl SlabNode {
    /// Get the path of the node in the slab.
    pub fn path(&self, slab: &Slab<SlabNode>) -> String {
        let mut segments = vec![self.name.clone()];
        // Write code like this to avoid the root node, which has no node name and shouldn't be put into semgents.
        if let Some(mut parent) = self.parent {
            while let Some(new_parent) = slab[parent].parent {
                segments.push(slab[parent].name.clone());
                parent = new_parent
            }
        }
        let mut result = String::new();
        for segment in segments.into_iter().rev() {
            result.push('/');
            result.push_str(&segment);
        }
        result
    }
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

fn construct_nodex_graph(parent: Option<usize>, node: &Node, slab: &mut Slab<SlabNode>) -> usize {
    let slab_node = SlabNode {
        parent,
        children: vec![],
        name: node.name.clone(),
    };
    let index = slab.insert(slab_node);
    slab[index].children = node
        .children
        .iter()
        .map(|node| construct_nodex_graph(Some(index), node, slab))
        .collect();
    index
}

/// Combine the construction routine of NamePool and BTreeMap since we can deduplicate node name for free.
// TODO(ldm0): Memory optimization can be done by letting name index reference the name in the pool(gc need to be considered though)
fn construct_name_index_and_namepool(
    slab: &Slab<SlabNode>,
    node_index: usize,
    name_index: &mut BTreeMap<String, Vec<usize>>,
    name_pool: &mut NamePool,
) {
    let node = &slab[node_index];
    if let Some(nodes) = name_index.get_mut(&node.name) {
        nodes.push(node_index);
    } else {
        name_pool.push(&node.name);
        name_index.insert(node.name.clone(), vec![node_index]);
    };
    for &node in &node.children {
        construct_name_index_and_namepool(slab, node, name_index, name_pool);
    }
}

fn main() {
    let (slab, slab_root) = {
        // first multithreaded walk the file system then get a simple tree structure
        let walk_data = WalkData::default();
        let visit_time = Instant::now();
        let node = walk_it(PathBuf::from("/"), &walk_data).expect("failed to walk");
        dbg!(walk_data);
        dbg!(visit_time.elapsed());

        // next construct the node graph which is single threaded but allows cross referencing
        let slab_time = Instant::now();
        let mut slab = Slab::new();
        let slab_root = construct_nodex_graph(None, &node, &mut slab);
        dbg!(slab_time.elapsed());
        dbg!(slab_root);
        dbg!(slab.len());
        (slab, slab_root)
    };

    {
        let name_index_time = Instant::now();
        let mut name_index = BTreeMap::default();
        let mut name_pool = NamePool::new();
        construct_name_index_and_namepool(&slab, slab_root, &mut name_index, &mut name_pool);
        dbg!(name_index_time.elapsed());
        dbg!(name_index.len());

        let search_time = Instant::now();
        for (i, name) in name_pool.search_substr("athbyt").enumerate() {
            // TODO(ldm0): this can be parallelized
            if let Some(nodes) = name_index.get(name) {
                for &node in nodes {
                    println!("[{}] {}", i, slab[node].path(&slab));
                }
            }
        }
        dbg!(name_pool.len() / 1024 / 1024);
        dbg!(search_time.elapsed());
    }

    {
        let cbor_time = Instant::now();
        let output = File::create("target/tree.cbor").unwrap();
        let mut output = BufWriter::new(output);
        cbor4ii::serde::to_writer(&mut output, &slab).unwrap();
        dbg!(cbor_time.elapsed());
        dbg!(fs::metadata("target/tree.cbor").unwrap().len() / 1024 / 1024);
    }

    {
        let bincode_time = Instant::now();
        let output = File::create("target/tree.bin").unwrap();
        let mut output = BufWriter::new(output);
        bincode::encode_into_std_write(&slab, &mut output, bincode::config::standard()).unwrap();
        dbg!(bincode_time.elapsed());
        dbg!(fs::metadata("target/tree.bin").unwrap().len() / 1024 / 1024);
    }

    {
        let zstd_bincode_time = Instant::now();
        let output = File::create("target/tree.bin.zstd").unwrap();
        let mut output = zstd::Encoder::new(output, 3).unwrap();
        bincode::encode_into_std_write(&slab, &mut output, bincode::config::standard()).unwrap();
        dbg!(zstd_bincode_time.elapsed());
        dbg!(fs::metadata("target/tree.bin.zstd").unwrap().len() / 1024 / 1024);
    }
}
