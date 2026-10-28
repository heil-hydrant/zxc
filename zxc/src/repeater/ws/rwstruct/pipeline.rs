use std::path::PathBuf;

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::Message;
use tracing::trace;

use super::RWebSocket;
use crate::file_types::{EXT_WREQ, EXT_WRES};
use crate::proxy::*;
use crate::repeater::prelude::*;

// Blanket Implementation
impl<T> AddRaw for RWebSocket<T> {
    fn add_raw(&mut self, _: BytesMut) {
        panic!("{}", WS_ADD_RAW_PANIC);
    }
}

impl<T> ReadWrite for RWebSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Error = WsError;
    type State = RepeaterState<Self>;

    /*  Transition:
     *      Receive -> WriteResponse
     *
     *  Error:
     *      WsError::Read
     */

    async fn read(mut self) -> Result<RepeaterState<Self>, WsError> {
        if let Some(msg) = self.stream.next().await {
            self.frame = Some(msg?);
        }
        trace!("Y");
        Ok(RepeaterState::WriteResponse(self))
    }

    // Transition: WriteResponse -> Receive | End
    async fn write(mut self) -> Result<RepeaterState<Self>, WsError> {
        let frame = self.frame.take().unwrap();
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

impl<T> RepeaterBytesToFrame for RWebSocket<T> {
    type Frame = Message;

    fn parse_frame(
        &mut self,
        buf: BytesMut,
    ) -> Result<Self::Frame, UpdateMsgError> {
        let msg = Message::Text(
            String::from_utf8_lossy(&buf)
                .to_string()
                .into(),
        );
        self.data = Some(buf);
        Ok(msg)
    }

    fn frame_to_payload(&mut self, payload: Self::Frame) {
        self.frame = Some(payload);
    }
}

impl<T> Rewrite for RWebSocket<T> {
    #[inline(always)]
    fn should_rewrite(&self) -> bool {
        false
    }

    // No rewrite for ws, so always return data None
    fn get_write_data_and_file(&mut self) -> (&[u8], &mut File) {
        panic!("No rewrite for ws");
    }
}

/* Steps:
 *        1. update path to id.wreq
 *        2. increment id
 */

impl<T> Newrite for RWebSocket<T> {
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

/* New ws frame should always be constructed.
 * So, always return true
 */

impl<T> ShouldUpdate for RWebSocket<T> {
    #[inline(always)]
    fn should_update(&self) -> bool {
        true
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
