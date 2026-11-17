mod pipeline;

use std::path::PathBuf;

use bytes::{Bytes, BytesMut};
use tokio::fs::File;
use tokio::sync::mpsc::Receiver;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tracing::trace;
use zxc_derive::FileBuf;

use crate::io::*;
use crate::proxy::handler_state::transition::write_history::WsHistory;
use crate::proxy::*;
use crate::repeater::prelude::*;

// RepeaterWs Handler
#[derive(FileBuf)]
pub struct RWebSocket<T> {
    pub stream: WebSocketStream<T>,
    pub receiver: Receiver<RepeaterWsMsg>,
    file: File, // Scratch file
    history: File,
    path: PathBuf,
    frame: Option<Message>,
    log_id: usize,
    buf: BytesMut,
    data: Option<Bytes>,
    is_bin: bool,
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
            is_bin: false,
        }
    }

    /* Description:
     *      Writes log to History file in format of:
     *            id | role.as_arrow() | size
     *            id | role.as_arrow() | size | b
     *
     * Steps:
     *      1. Get size and binary flag,
     *          Server => data is stored as self.data
     *          Client => data is stored as self.frame
     *
     *      2. Build data in format of:
     *              id | as_arrow(role) | size [| b]
     *
     * Error:
     *      FileErrorInfo
     */

    pub async fn log(&mut self, role: Role) -> Result<(), FileErrorInfo> {
        let (size, is_bin) = match role {
            Role::Server => (self.data_as_ref().len(), self.is_bin),
            Role::Client => (
                self.response_data().len(),
                self.frame.as_ref().is_some_and(Message::is_binary),
            ),
        };

        let data =
            WsHistory::new(self.log_id, &role, is_bin, size).to_string();
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

    pub fn set_is_bin(&mut self, is_bin: bool) {
        self.is_bin = is_bin;
    }

    pub fn file_as_mut(&mut self) -> &mut File {
        &mut self.file
    }
}
