//! Addons window backend.
//!
//! Copies a forwarded request into the session `addons` directory, rewrites
//! HTTP/2 requests into HTTP/1.1, and emits the configured external tool
//! command to the Vim UI over `addons.sock`. The source request file is never
//! modified; only the copy is handed to the tool.

mod h2;

pub mod builder;
pub mod error;
pub mod handler;
pub mod message;

pub use error::AddonError;
pub use handler::AddonHandler;
