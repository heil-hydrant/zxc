pub mod error;
pub mod reader;
pub mod writer;

pub use error::UnixSockError;
pub use reader::read_from_unix;
pub use writer::{
    CLOSE_SQUARE_BRACKET, DEFAULT_ID, build_slice_and_write,
    unix_write_vec_and_flush,
};
