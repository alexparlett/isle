//! Providers: programs that answer in JSON, run sandboxed with what the user granted (B14,
//! B28; `platform.md` Providers).

pub mod grant;
pub mod manifest;
pub mod proxy;
pub mod run;
pub mod sandbox;
pub mod sources;
pub mod template;

pub use grant::Grant;
pub use manifest::Manifest;
pub use template::Data;
