// Phase 3: 快照与对比模块;
mod serialize;
mod diff;
#[cfg(feature = "snapshot")]
mod storage;

pub use diff::{ChangeType, DiffNode, diff_level};
#[cfg(feature = "snapshot")]
pub use storage::{SnapshotMeta, SnapshotStorage};
