use std::fmt::Display;

use bytes::BytesMut;

use super::transition::*;
use crate::async_step::AsyncStep;
use crate::proxy::handler_state::transition::FileBuf;
use crate::proxy::*;

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
    T: FileBuf
        + ShouldUpdate
        + AddRaw
        + RepeaterBytesToFrame
        + Persist
        + ReadWrite
        + WriteResponse
        + ReadWrite<State = Self>,
    <T as ReadWrite>::Error: Into<ProtocolError>,
{
    type Error = ProtocolError;

    async fn next(self) -> Result<Self, Self::Error> {
        match self {
            // Reads the payload and applies the update policy.
            //
            // # Transitions
            //
            // `ReadFromFile` -> `UpdateFrame` | `Send`.
            Self::ReadFromFile(conn) => read_from_file(conn).await,
            // Converts the buffered bytes into a frame.
            Self::UpdateFrame(mut conn, buf) => {
                let frame = conn.parse_frame(buf)?;
                conn.frame_to_payload(frame);
                Ok(RepeaterState::ReWrite(conn))
            }
            // Persists the payload to disk.
            Self::ReWrite(mut conn) => {
                conn = rewrite(conn).await?;
                Ok(Self::Send(conn))
            }
            // Sends the request.
            Self::Send(conn) => conn.write().await.map_err(Into::into),
            // Reads the response or frame from the server.
            Self::Receive(conn) => conn.read().await.map_err(Into::into),
            // Logs the response to disk.
            Self::WriteResponse(mut conn) => {
                conn = log_response(conn).await?;
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
