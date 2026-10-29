//! Providers: programs that answer in JSON, run sandboxed with what the user granted (B14,
//! B28; `platform.md` Providers).

pub mod grant;
pub mod installed;
pub mod manifest;
pub mod proxy;
#[cfg(feature = "sources")]
pub mod release;
pub mod run;
pub mod sandbox;
#[cfg(feature = "sources")]
pub mod sources;
pub mod template;

pub use grant::Grant;
pub use manifest::Manifest;
pub use template::Data;
