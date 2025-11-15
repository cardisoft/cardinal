use fswalk::NodeFileType;
use serde::{
    Deserialize, Serialize,
    de::{self, SeqAccess, Visitor},
    ser::SerializeTuple,
};
use std::{fmt, num::NonZeroU32};
use thin_vec::ThinVec;

#[derive(Debug, Clone, Copy)]
pub struct NameAndParent {
    ptr: *const u8,
    // Length of the filename should not be larger than 256 chars(macOS, Linux,
    // Window, BSD) should be enough.
    len: u32,
    parent: crate::OptionSlabIndex,
}

unsafe impl Send for NameAndParent {}

impl Serialize for NameAndParent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_tuple(2)?;
        s.serialize_element(self.as_str())?;
        s.serialize_element(&self.parent)?;
        s.end()
    }
}

impl<'de> serde::de::Deserialize<'de> for NameAndParent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct NameAndParentVisitor;

        impl<'de> Visitor<'de> for NameAndParentVisitor {
            type Value = NameAndParent;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a tuple of (string, OptionSlabIndex)")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let name: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let parent: crate::OptionSlabIndex = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                let name_in_pool = crate::NAME_POOL.push(&name);

                Ok(NameAndParent::new(name_in_pool, parent))
            }
        }

        deserializer.deserialize_tuple(2, NameAndParentVisitor)
    }
}

impl std::ops::Deref for NameAndParent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl NameAndParent {
    pub fn new(s: &'static str, parent: crate::OptionSlabIndex) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s
                .len()
                .try_into()
                .expect("get filename larger than 256 bytes"),
            parent,
        }
    }

    pub fn as_str(&self) -> &'static str {
        unsafe { std::str::from_raw_parts(self.ptr, self.len as usize) }
    }

    pub fn parent(&self) -> Option<crate::SlabIndex> {
        self.parent.to_option()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlabNode {
    pub name_and_parent: NameAndParent,
    pub children: ThinVec<crate::SlabIndex>,
    pub metadata: SlabNodeMetadataCompact,
}

impl SlabNode {
    pub fn add_children(&mut self, children: crate::SlabIndex) {
        if !self.children.contains(&children) {
            self.children.push(children);
        }
    }

    pub fn new(
        parent: Option<crate::SlabIndex>,
        name: &'static str,
        metadata: SlabNodeMetadataCompact,
    ) -> Self {
        Self {
            name_and_parent: NameAndParent::new(name, crate::OptionSlabIndex::from_option(parent)),
            children: ThinVec::new(),
            metadata,
        }
    }
}

/// SlabNodeMetadataCompact with state ensured to be Some
pub struct SlabNodeMetadata<'a>(&'a SlabNodeMetadataCompact);

impl<'a> SlabNodeMetadata<'a> {
    pub fn r#type(&self) -> fswalk::NodeFileType {
        self.0.state_type_and_size.r#type()
    }

    pub fn size(&self) -> u64 {
        self.0.state_type_and_size.size()
    }

    pub fn ctime(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.0.ctime)
    }

    pub fn mtime(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.0.mtime)
    }
}

/// Use a compact form so that
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct SlabNodeMetadataCompact {
    state_type_and_size: crate::StateTypeSize,
    // Actually a Option<NonZeroU32>, but using u32 here due to https://github.com/serde-rs/serde/issues/1834
    ctime: u32,
    mtime: u32,
}

impl SlabNodeMetadataCompact {
    pub fn unaccessible() -> Self {
        Self {
            state_type_and_size: crate::StateTypeSize::unaccessible(),
            ctime: 0,
            mtime: 0,
        }
    }

    pub fn some(
        fswalk::NodeMetadata {
            r#type,
            size,
            ctime,
            mtime,
        }: fswalk::NodeMetadata,
    ) -> Self {
        Self {
            state_type_and_size: crate::StateTypeSize::some(r#type, size),
            ctime: ctime
                .and_then(|x| std::num::NonZeroU32::try_from(x).ok())
                .map(|x| x.get())
                .unwrap_or_default(),
            mtime: mtime
                .and_then(|x| std::num::NonZeroU32::try_from(x).ok())
                .map(|x| x.get())
                .unwrap_or_default(),
        }
    }

    pub fn none() -> Self {
        Self {
            state_type_and_size: crate::StateTypeSize::none(),
            ctime: 0,
            mtime: 0,
        }
    }

    pub fn state(&self) -> crate::State {
        self.state_type_and_size.state()
    }

    pub fn as_ref(&self) -> Option<SlabNodeMetadata<'_>> {
        match self.state() {
            crate::State::Some => Some(SlabNodeMetadata(self)),
            crate::State::Unaccessible | crate::State::None => None,
        }
    }

    pub fn is_some(&self) -> bool {
        matches!(self.state(), crate::State::Some)
    }

    pub fn is_none(&self) -> bool {
        matches!(self.state(), crate::State::None)
    }

    pub fn is_unaccessible(&self) -> bool {
        matches!(self.state(), crate::State::Unaccessible)
    }

    pub fn file_type_hint(&self) -> NodeFileType {
        self.state_type_and_size.r#type()
    }

    pub fn size_hint(&self) -> Option<u64> {
        (self.state() == crate::State::Some).then(|| self.state_type_and_size.size())
    }
}

#[derive(Debug)]
pub struct SearchResultNode {
    pub path: std::path::PathBuf,
    pub metadata: SlabNodeMetadataCompact,
}
