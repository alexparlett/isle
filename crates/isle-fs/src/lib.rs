//! Files' engine, with no UI: folders listed in batches, kept in order and watched, the
//! mount table and its probes (`p15-files.md` choices 8–13). Nothing here may run on a
//! thread that draws.

pub mod copy;
pub mod entry;
pub mod folder;
pub mod jobs;
pub mod list;
pub mod mounts;
pub mod order;
pub mod trash;
pub mod watch;

pub use entry::{Entry, Kind};
pub use folder::{Applied, Folder};
pub use list::{ListError, Listed, list, spawn as spawn_listing};
pub use order::{Order, SortBy, natural_key};
