use std::path::{Path, PathBuf};

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tracing::trace;

use super::RWebSocket;
use crate::file_types::{EXT_WREQ, EXT_WRES};
use crate::proxy::handler_state::WS_ADD_RAW_PANIC;
use crate::proxy::*;
use crate::repeater::prelude::*;

// WebSocket frames are always new, so this always returns `true`.
impl<T> ShouldUpdate for RWebSocket<T> {
    #[inline(always)]
    fn should_update(&self) -> bool {
        true
    }
}

// Blanket Implementation
impl<T> AddRaw for RWebSocket<T> {
    fn add_raw(&mut self, _: BytesMut) -> Result<(), UpdateMsgError> {
        panic!("{}", WS_ADD_RAW_PANIC);
    }
}

impl<T> RepeaterBytesToFrame for RWebSocket<T> {
    type Frame = Message;

    fn parse_frame(
        &mut self,
        buf: BytesMut,
    ) -> Result<Self::Frame, UpdateMsgError> {
        let bytes = buf.freeze();
        let msg = if self.is_bin {
            Message::Binary(bytes.clone())
        } else {
            match Utf8Bytes::try_from(bytes.clone()) {
                Ok(utf8) => Message::Text(utf8),
                Err(_) => Message::Binary(bytes.clone()),
            }
        };
        // TODO: needed ?
        self.is_bin = msg.is_binary();
        self.data = Some(bytes);
        Ok(msg)
    }

    fn frame_to_payload(&mut self, payload: Self::Frame) {
        self.frame = Some(payload);
    }
}

impl<T> Persist for RWebSocket<T> {
    #[inline(always)]
    fn should_rewrite(&self) -> bool {
        false
    }

    // No rewrite for ws, so always return data None
    fn file_data_and_path(&mut self) -> (&mut File, &[u8], &Path) {
        panic!("No rewrite for ws");
    }

    // Updates the path to `id.wreq` and increments the ID.
    fn update_path(&mut self) {
        self.path
            .set_file_name(self.log_id.to_string());
        self.path.set_extension(EXT_WREQ);
    }

    fn data_as_ref(&self) -> &[u8] {
        self.data.as_ref().unwrap()
    }

    fn path_as_ref(&self) -> &PathBuf {
        &self.path
    }
}

impl<T> ReadWrite for RWebSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Error = WsError;
    type State = RepeaterState<Self>;

    async fn read(mut self) -> Result<RepeaterState<Self>, WsError> {
        let Some(msg) = self.stream.next().await else {
            return Err(WsError::Close);
        };
        self.frame = Some(msg?);
        trace!("Y");
        Ok(RepeaterState::WriteResponse(self))
    }

    // Transitions `WriteResponse` to `Receive` or `End`.
    async fn write(mut self) -> Result<RepeaterState<Self>, WsError> {
        let frame = self
            .frame
            .take()
            .ok_or(WsError::NoMsg)?;
        let is_close = frame.is_close();
        self.stream
            .send(frame)
            .await
            .map_err(|_| WsError::Write)?;
        trace!("Y");
        if !is_close {
            trace!("continue");
            return Ok(RepeaterState::Receive(self));
        }
        trace!("close frame");
        Ok(RepeaterState::End(self))
    }
}

impl<T> WriteResponse for RWebSocket<T> {
    // filename: log_id.wres
    fn update_response_path(&mut self) {
        self.path
            .set_file_name(self.log_id.to_string());
        self.path.set_extension(EXT_WRES);
    }

    fn response_path(&self) -> &PathBuf {
        &self.path
    }

    fn response_data(&self) -> &[u8] {
        // safe to unwrap
        match self.frame.as_ref().unwrap() {
            Message::Text(frame) => frame.as_bytes(),
            Message::Binary(frame) => frame,
            _ => &[],
        }
    }
}
