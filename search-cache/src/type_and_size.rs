use fswalk::NodeFileType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
#[serde(transparent)]
/// state in the high 2 bits, type in the next 2bits, size in the low 60 bits
pub struct StateTypeSize(u64);

impl StateTypeSize {
    pub fn none() -> Self {
        assert_eq!(NodeFileType::File as u8, 0);
        Self::new(State::None, NodeFileType::File, 0)
    }

    pub fn unaccessible() -> Self {
        assert_eq!(NodeFileType::File as u8, 0);
        Self::new(State::Unaccessible, NodeFileType::File, 0)
    }

    pub fn some(r#type: NodeFileType, size: u64) -> Self {
        Self::new(State::Some, r#type, size)
    }

    fn new(state: State, r#type: NodeFileType, size: u64) -> Self {
        Self(size.min((1 << 60) - 1) | ((r#type as u64) << 60) | ((state as u64) << 62))
    }

    pub fn state(&self) -> State {
        State::n((self.0 >> 62) as u8).unwrap()
    }

    pub fn r#type(&self) -> NodeFileType {
        NodeFileType::n((self.0 >> 60 & 0b11) as u8).unwrap()
    }

    /// Returns the size in bytes, or -1 for directories.
    ///
    /// Directories return -1 to facilitate proper sorting where directories
    /// should appear before or after files based on sort direction.
    pub fn size(&self) -> i64 {
        if self.r#type() == NodeFileType::Dir {
            -1
        } else {
            (self.0 & ((1u64 << 60) - 1)) as i64
        }
    }
}

#[derive(Debug, Clone, Copy, enumn::N, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Unaccessible = 0,
    Some = 1,
    None = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_and_size() {
        let max_size = (1u64 << 60) - 1;
        let state = State::Some;
        let file_type = NodeFileType::File;
        let ts = StateTypeSize::new(state, file_type, max_size);
        assert_eq!(ts.state(), State::Some);
        assert_eq!(ts.r#type(), file_type);
        assert_eq!(ts.size(), max_size as i64);

        let file_type = NodeFileType::Dir;
        let size = 12345;
        let ts = StateTypeSize::new(state, file_type, size);
        assert_eq!(ts.state(), State::Some);
        assert_eq!(ts.r#type(), file_type);
        assert_eq!(ts.size(), -1);

        let state = State::None;
        let file_type = NodeFileType::Symlink;
        let size = 0;
        let ts = StateTypeSize::new(state, file_type, size);
        assert_eq!(ts.state(), State::None);
        assert_eq!(ts.r#type(), file_type);
        assert_eq!(ts.size(), size as i64);

        let state = State::Unaccessible;
        let file_type = NodeFileType::Unknown;
        let size = 987654321;
        let ts = StateTypeSize::new(state, file_type, size);
        assert_eq!(ts.state(), State::Unaccessible);
        assert_eq!(ts.r#type(), file_type);
        assert_eq!(ts.size(), size as i64);
    }

    #[test]
    fn test_size_overflow() {
        let too_large_size = 1u64 << 60;
        let state = State::Some;
        let file_type = NodeFileType::File;
        let ts = StateTypeSize::new(state, file_type, too_large_size);
        assert_eq!(ts.state(), State::Some);
        assert_eq!(ts.r#type(), file_type);
        assert_eq!(ts.size(), ((1u64 << 60) - 1) as i64); // size saturating

        let another_large_size = ((1u64 << 60) - 1) + 100;
        let ts = StateTypeSize::new(state, file_type, another_large_size);
        assert_eq!(ts.state(), State::Some);
        assert_eq!(ts.r#type(), file_type);
        assert_eq!(ts.size(), ((1u64 << 60) - 1) as i64);

        let max_size = (1 << 60) - 1;
        let ts = StateTypeSize::new(state, file_type, max_size);
        assert_eq!(ts.state(), State::Some);
        assert_eq!(ts.r#type(), file_type);
        assert_eq!(ts.size(), max_size as i64);
    }
}
