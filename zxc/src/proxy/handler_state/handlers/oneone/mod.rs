mod connection;
mod error;
mod handle;

pub use connection::OneOneConn;
pub use connection::OneOneRWError;
pub use handle::{handle_http, read_http};

// Compatibility shim for the old module path.
pub mod oneoneconn {
    pub use super::connection::OneOneConn;
}
