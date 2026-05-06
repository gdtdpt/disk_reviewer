// Phase 3: 快照与对比模块;
mod serialize;
mod diff;
#[cfg(feature = "snapshot")]
mod storage;

pub use serialize::{deserialize_tree, serialize_tree};
pub use diff::{ChangeType, DiffNode, diff_level, diff_trees_recursive, entry_name};
#[cfg(feature = "snapshot")]
pub use storage::{SnapshotMeta, SnapshotStorage};
