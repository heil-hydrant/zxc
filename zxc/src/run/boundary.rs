use std::fmt::Display;
use std::io;

use bytes::BytesMut;
use thiserror::Error;
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
    ) -> Result<UiReply, Self::Error>;
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
    ) -> Result<UiReply, <Self as HandleCommander>::Error>;
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
    fn is_ui_error(&self) -> bool {
        false
    }
}

/* Description:
 *      Reply returned by UI handlers.
 *
 *      None  : nothing to send.
 *      Reply : send data to UI. id 0 is a push, any other id is an RPC reply.
 *      Flush : flush queued storage to the UI.
 */
pub enum UiReply {
    None,
    Reply(usize, String),
    Flush,
}

impl UiReply {
    pub fn push(data: String) -> Self {
        Self::Reply(0, data)
    }
}

/* Description:
 *      Errors shared by all UI handlers.
 *
 *      Only Socket errors terminate the UI runner. NoUI, Closed and Message
 *      errors are logged and the runner continues.
 */
#[derive(Debug, Error)]
pub enum UiError {
    #[error("no UI connected")]
    NoUI,
    #[error("UI closed")]
    Closed,
    #[error("UI message error: {0}")]
    Message(#[from] serde_json::Error),
    #[error("ui socket: {0}")]
    Socket(#[from] UnixSockError),
}

impl IsUIError for UiError {
    fn is_ui_error(&self) -> bool {
        matches!(self, UiError::Socket(_))
    }
}

/* Description:
 *      Generates the shared UI error conversions for a handler error enum.
 *
 *      The enum must have a `Ui(UiError)` variant.
 */
#[macro_export]
macro_rules! impl_ui_error {
    ($err:ty) => {
        impl From<$crate::io::UnixSockError> for $err {
            fn from(e: $crate::io::UnixSockError) -> Self {
                Self::Ui($crate::run::boundary::UiError::Socket(e))
            }
        }

        impl From<serde_json::Error> for $err {
            fn from(e: serde_json::Error) -> Self {
                Self::Ui($crate::run::boundary::UiError::Message(e))
            }
        }

        impl $crate::run::boundary::IsUIError for $err {
            fn is_ui_error(&self) -> bool {
                matches!(
                    self,
                    Self::Ui(e)
                        if $crate::run::boundary::IsUIError::is_ui_error(e)
                )
            }
        }
    };
}
