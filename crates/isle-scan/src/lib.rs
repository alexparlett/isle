//! Where a disk's space went, for Cleaner and Settings' Storage (`p19b-cleaner.md`).

pub mod duplicates;
pub mod kinds;
pub mod ondisk;
pub mod rules;
pub mod system;
pub mod tree;
pub mod walk;

pub use kinds::Kind;
pub use rules::{Context, Find, Rules};
pub use tree::{File, Id, TOP, Tree};
pub use walk::{Disk, Progress, Scan, all_disks};
