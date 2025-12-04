use crate::{SlabIndex, SlabNode, ThinSlab};
use std::{
    ffi::OsStr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct FileNodes {
    path: PathBuf,
    slab: ThinSlab<SlabNode>,
    root: SlabIndex,
}

impl FileNodes {
    pub(crate) fn new(path: PathBuf, slab: ThinSlab<SlabNode>, root: SlabIndex) -> Self {
        Self { path, slab, root }
    }

    pub(crate) fn root(&self) -> SlabIndex {
        self.root
    }

    pub fn node_path(&self, index: SlabIndex) -> Option<PathBuf> {
        let mut current = index;
        let mut segments = vec![];
        while let Some(parent) = self.slab.get(current)?.parent() {
            segments.push(self.slab.get(current)?.name());
            current = parent;
        }
        Some(
            self.path
                .iter()
                .chain(segments.iter().rev().map(OsStr::new))
                .collect(),
        )
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn into_parts(self) -> (PathBuf, SlabIndex, ThinSlab<SlabNode>) {
        let Self { path, slab, root } = self;
        (path, root, slab)
    }
}

impl Deref for FileNodes {
    type Target = ThinSlab<SlabNode>;

    fn deref(&self) -> &Self::Target {
        &self.slab
    }
}

impl DerefMut for FileNodes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slab
    }
}
