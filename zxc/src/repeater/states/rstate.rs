use std::fmt::{Display, Formatter};

use bytes::BytesMut;

use super::transition::bytes_to_frame::RepeaterBytesToFrame;
use super::transition::read_from_file::{RepeaterReadFile, read_from_file};
use super::transition::rewrite::{Newrite, Rewrite, rewrite};
use super::transition::should_update::ShouldUpdate;
use super::transition::write_response::{WriteResponse, log_response};
use crate::async_step::AsyncStep;
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::handler_state::read_write::ReadWrite;
use crate::proxy::handler_state::transition::read_modified_file::AddRaw;

// All possible Repeater States
// In order
pub enum RepeaterState<T> {
    ReadFromFile(T),
    UpdateFrame(T, BytesMut),
    ReWrite(T),
    Send(T),
    Receive(T),
    WriteResponse(T),
    End(T),
}

impl<T> AsyncStep for RepeaterState<T>
where
    T: RepeaterReadFile
        + ShouldUpdate
        + AddRaw
        + RepeaterBytesToFrame
        + Rewrite
        + Newrite
        + ReadWrite
        + WriteResponse
        + ReadWrite<State = Self>,
    <T as ReadWrite>::Error: Into<ProxyStateError>,
{
    type Error = ProxyStateError;

    async fn next(self) -> Result<Self, Self::Error> {
        match self {
            /* Transition:
             *      ReadFromFile -> UpdateFrame | Send
             *
             * Trait Bound:
             *      RepeaterReadFile + ShouldUpdate + AddRaw
             *
             * Errors:
             *      ProxyStateError::FileIo
             */
            Self::ReadFromFile(conn) => read_from_file(conn)
                .await
                .map_err(ProxyStateError::FileIo),

            /* Transition:
             *      UpdateFrame -> ReWrite
             *
             * Trait Bound:
             *      RepeaterBytesToFrame
             *
             * Errors:
             *      ProxyStateError::UpdateFrame
             */
            Self::UpdateFrame(mut conn, buf) => {
                let frame = conn.parse_frame(buf)?;
                conn.frame_to_payload(frame);
                Ok(RepeaterState::ReWrite(conn))
            }
            /* Transition:
             *      ReWrite -> Send
             *
             * Trait Bound:
             *      Rewrite + Newrite,
             *
             * Errors:
             *      ProxyStateError::RewriteFile
             */
            Self::ReWrite(mut conn) => {
                conn = rewrite(conn)
                    .await
                    .map_err(ProxyStateError::ReWriteFile)?;
                Ok(Self::Send(conn))
            }

            /* Transition:
             *      Send -> Receive
             *
             * Trait Bound:
             *      ReadWrite
             *
             * Errors:
             *      RepeaterError::ReadHttp | RepeaterError::WsIo
             */
            Self::Send(conn) => conn.write().await.map_err(Into::into),

            /* Transition:
             *      Receive -> WriteResponse
             *
             * Trait Bound:
             *      ReadWrite
             *
             * Errors:
             *      RepeaterError::ReadHttp | RepeaterError::WsIo
             */
            Self::Receive(conn) => conn.read().await.map_err(Into::into),

            /* Transition:
             *      WriteResponse -> End
             *
             * Trait Bound:
             *      WriteResponse
             *
             * Errors:
             *      RepeaterError::LogResponse
             */
            Self::WriteResponse(mut conn) => {
                conn = log_response(conn)
                    .await
                    .map_err(ProxyStateError::ReWriteFile)?;
                Ok(Self::End(conn))
            }
            Self::End(conn) => Ok(Self::End(conn)),
        }
    }

    // Check if the state machine has ended
    fn is_ended(&self) -> bool {
        matches!(self, Self::End(_))
    }
}

impl<T> Display for RepeaterState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::ReadFromFile(_) => "read_from_file",
            Self::UpdateFrame(..) => "update_frame",
            Self::ReWrite(_) => "rewrite",
            Self::Send(_) => "send",
            Self::Receive(_) => "receive",
            Self::WriteResponse(_) => "write_response",
            Self::End(_) => "end",
        };
        write!(f, "{}", s)
    }
}
