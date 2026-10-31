//! Where a disk's space went, for Cleaner and Settings' Storage (`p19b-cleaner.md`).

pub mod kinds;
pub mod tree;
pub mod walk;

pub use kinds::Kind;
pub use tree::{File, Id, TOP, Tree};
pub use walk::{Disk, Progress, Scan, all_disks};
