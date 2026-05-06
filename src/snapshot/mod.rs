// Phase 3: 快照与对比模块
mod serialize;
#[cfg(feature = "snapshot")]
mod storage;

pub use serialize::{deserialize_tree, serialize_tree};
#[cfg(feature = "snapshot")]
pub use storage::{SnapshotMeta, SnapshotStorage};
