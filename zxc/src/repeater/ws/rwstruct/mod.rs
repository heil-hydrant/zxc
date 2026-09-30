use std::path::PathBuf;

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Receiver;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tracing::trace;
use zxc_derive::RepeaterReadFile;

use super::{RepeaterState, RepeaterWsMsg};
use crate::io::file::{FileErrorInfo, FileEvent};
use crate::io::write::write_and_flush;
use crate::proxy::handler_state::handlers::error::WsError;
use crate::proxy::handler_state::read_write::ReadWrite;
use crate::proxy::handler_state::role::Role;
use crate::repeater::states::transition::read_from_file::RepeaterReadFile;
use crate::repeater::states::transition::rewrite::Newrite;
use crate::repeater::states::transition::write_response::WriteResponse;
mod impl_add_raw;
mod impl_read_write;
mod impl_repeater_bytes_to_frame;
mod impl_rewrite;
mod impl_should_update;
mod impl_write_response;

// RepeaterWs Handler
#[derive(RepeaterReadFile)]
pub struct RWebSocket<T> {
    pub stream: WebSocketStream<T>,
    pub receiver: Receiver<RepeaterWsMsg>,
    file: File, // Scratch file
    history: File,
    path: PathBuf,
    frame: Option<Message>,
    log_id: usize,
    buf: BytesMut,
    data: Option<BytesMut>,
}

impl<T> RWebSocket<T> {
    pub fn new(
        stream: WebSocketStream<T>,
        receiver: Receiver<RepeaterWsMsg>,
        path: PathBuf,
        file: File,
        buf: BytesMut,
        history: File,
    ) -> Self {
        RWebSocket {
            stream,
            receiver,
            file,
            path,
            frame: None,
            log_id: 1,
            buf,
            history,
            data: None,
        }
    }

    /* Description:
     *      Writes log to History file in format of:
     *            id | role.as_ws_arrow() | size
     *
     * Steps:
     *      1. Get size,
     *          Server => data is stored as self.frame
     *          Client => data is response_data
     *
     *      2. Build data in format of:
     *              id | as_arrow(role) | size
     *
     * Error:
     *      FileErrorInfo
     */

    pub async fn log(&mut self, role: Role) -> Result<(), FileErrorInfo> {
        let size = match role {
            Role::Server => self.data_as_ref().len(),
            Role::Client => self.response_data().len(),
        };

        let data =
            format!("{} | {} | {}\n", self.log_id, role.as_arrow(), size);
        trace!("log| {}", data);
        // increment log_id for next write
        self.log_id += 1;
        write_and_flush(&mut self.history, data.as_bytes())
            .await
            .map_err(|e: std::io::Error| {
                FileErrorInfo::from((&mut self.history, FileEvent::Write, e))
            })
    }

    pub fn add_frame(&mut self, frame: Message) {
        self.frame = Some(frame);
    }

    pub fn file_as_mut(&mut self) -> &mut File {
        &mut self.file
    }
}
