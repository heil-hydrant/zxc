use std::fmt::Display;
use std::io;

use bytes::BytesMut;
use tokio::net::UnixStream;

use crate::io::UnixSockError;

// The Following traits are implemented by the ui handlers.
//
//     AddonHandler
//     HistoryHandler
//     InterceptorHandler
//     RepeaterHandler

pub trait FromCommander {
    type Message;

    async fn recv(&mut self) -> Option<Self::Message>;
}

pub trait HandleCommander: FromCommander {
    type Error: Display + From<UnixSockError> + IsUIError;

    async fn handle_commander_no_ui(
        &mut self,
        msg: <Self as FromCommander>::Message,
    ) -> Result<(), Self::Error>;

    async fn handle_commander_ui(
        &mut self,
        msg: <Self as FromCommander>::Message,
    ) -> Result<Option<String>, Self::Error>;
}

/* Description:
 *      Flush the storage to unix socket.
 *
 * Actual implementation:
 *      HistoryHandler
 *
 * All other handlers blanket implementation using derive macro.
 *      zxc-derive/src/flush_storage.rs
 */
pub trait FlushStorage {
    async fn flush_storage(
        &mut self,
        stream: &mut UnixStream,
    ) -> Result<(), UnixSockError>;
}

/* Description:
 *      When the binary is closed and there is remaining data that needs to be
 *      written, write it to a .state file.
 *
 * Actual implementation:
 *      HistoryHandler
 *
 * All other handlers blanket implementation using derive macro.
 *      zxc-derive/src/close_action.rs
 */
pub trait CloseAction {
    async fn close_action(&mut self) -> Result<(), io::Error>;
}

// implemented as derive macro in zxc-derive/src/buffer.rs
pub trait Buffer {
    fn buf_as_mut(&mut self) -> &mut BytesMut;
}

pub trait HandleUI: HandleCommander {
    async fn handle_ui(
        &mut self,
    ) -> Result<Option<(usize, String)>, <Self as HandleCommander>::Error>;
}

/* Description:
 *      When the ui is closed/error, notify the commander so that no messages
 *      are received.
 *
 * Actual implementation:
 *      HistoryHandler
 *
 * All other handlers blanket implementation using derive macro.
 *      zxc-derive/src/notify.rs
 */

pub trait NotifyCommander: HandleCommander {
    async fn notify_commander(
        &mut self,
    ) -> Result<(), <Self as HandleCommander>::Error>;
}

pub trait IsUIError {
    fn is_ui_error(&self) -> bool;

    fn needs_flush(&self) -> bool;
}
