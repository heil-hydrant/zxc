/* Traits required for repeater state transition
 *
 * Implementors:
 *      - Roneone
 *      - RWebsocket
 */

pub mod bytes_to_frame;
pub mod read_from_file;
pub mod rewrite;
pub mod should_update;
pub mod write_response;

pub use bytes_to_frame::RepeaterBytesToFrame;
pub use read_from_file::{RepeaterReadFile, read_from_file};
pub use rewrite::{Newrite, Rewrite, rewrite};
pub use should_update::ShouldUpdate;
pub use write_response::{WriteResponse, log_response};
