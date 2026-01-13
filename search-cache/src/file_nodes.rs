use crate::{SlabIndex, SlabNode, ThinSlab};
use std::{
    ffi::OsStr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct FileNodes {
    path: PathBuf,
    ignore_paths: Vec<PathBuf>,
    slab: ThinSlab<SlabNode>,
    root: SlabIndex,
}

impl FileNodes {
    pub(crate) fn new(
        path: PathBuf,
        ignore_paths: Vec<PathBuf>,
        slab: ThinSlab<SlabNode>,
        root: SlabIndex,
    ) -> Self {
        Self {
            path,
            ignore_paths,
            slab,
            root,
        }
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
            std::iter::once("/")
                .chain(segments.into_iter().rev())
                .map(OsStr::new)
                .collect(),
        )
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn ignore_paths(&self) -> &Vec<PathBuf> {
        &self.ignore_paths
    }

    pub(crate) fn take_slab(&mut self) -> ThinSlab<SlabNode> {
        std::mem::take(&mut self.slab)
    }

    pub(crate) fn put_slab(&mut self, slab: ThinSlab<SlabNode>) {
        self.slab = slab;
    }

    pub(crate) fn into_parts(self) -> (PathBuf, Vec<PathBuf>, SlabIndex, ThinSlab<SlabNode>) {
        let Self {
            path,
            ignore_paths,
            slab,
            root,
        } = self;
        (path, ignore_paths, root, slab)
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
