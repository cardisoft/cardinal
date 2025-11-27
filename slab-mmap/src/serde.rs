use super::{Slab, builder::Builder};
use crate::INITIAL_SLOTS;
use core::{fmt, marker::PhantomData};
use serde::{
    de::{Deserialize, Deserializer, Error as DeError, MapAccess, Visitor},
    ser::{Serialize, SerializeMap, Serializer},
};
use std::num::NonZeroUsize;

impl<T> Serialize for Slab<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map_serializer = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in self {
            map_serializer.serialize_key(&key)?;
            map_serializer.serialize_value(value)?;
        }
        map_serializer.end()
    }
}

struct SlabVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for SlabVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Slab<T>;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let size = map.size_hint().unwrap_or_default();
        let size = NonZeroUsize::new(size).unwrap_or(INITIAL_SLOTS);
        let mut builder = Builder::with_capacity(size).map_err(A::Error::custom)?;

        while let Some((key, value)) = map.next_entry()? {
            builder.pair(key, value).map_err(A::Error::custom)?
        }

        Ok(builder.build())
    }
}

impl<'de, T> Deserialize<'de> for Slab<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SlabVisitor(PhantomData))
    }
}
