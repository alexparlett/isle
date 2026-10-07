//! Notification banners: the stack at the top right of the main screen
//! (`p1/structure/notifications.md`, `p1/surfaces/notifications.md`).

pub mod model;
pub mod surface;

pub use surface::{BannerAction, BannersSurface};
