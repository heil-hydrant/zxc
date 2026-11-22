//! Repeater window backend.
//!
//! Replays HTTP/1.1 requests and WebSocket frames, including the WebSocket
//! upgrade path, session files, and synchronous UI RPC replies over
//! `repeater.sock`.
pub mod conn;
pub mod error;
pub mod file_builder;
pub mod h2;
pub mod handler;
pub mod http;
pub mod messages;
pub mod prelude;
pub mod states;
pub mod ws;

pub use handler::RepeaterHandler;
