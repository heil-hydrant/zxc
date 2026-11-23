mod pipeline;

use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use bytes::{Bytes, BytesMut};
use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use tokio::sync::mpsc::Receiver;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tracing::trace;

use crate::io::*;
use crate::proxy::handler_state::transition::write_history::WsHistory;
use crate::proxy::*;
use crate::repeater::prelude::*;

// RepeaterWs Handler
pub struct RWebSocket<T> {
    pub stream: WebSocketStream<T>,
    pub receiver: Receiver<RepeaterWsMsg>,
    file: File, // Scratch file
    scratch_path: PathBuf,
    history: File,
    history_path: PathBuf,
    path: PathBuf,
    frame: Option<Message>,
    log_id: usize,
    buf: BytesMut,
    data: Option<Bytes>,
    is_bin: bool,
}

impl<T> FileBuf for RWebSocket<T> {
    fn file_and_buf_as_mut(&mut self) -> (&mut File, &mut BytesMut) {
        (&mut self.file, &mut self.buf)
    }

    fn file_path(&self) -> &Path {
        self.scratch_path.as_path()
    }
}

impl<T> RWebSocket<T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stream: WebSocketStream<T>,
        receiver: Receiver<RepeaterWsMsg>,
        path: PathBuf,
        scratch_path: PathBuf,
        file: File,
        buf: BytesMut,
        history_path: PathBuf,
        history: File,
    ) -> Self {
        RWebSocket {
            stream,
            receiver,
            file,
            scratch_path,
            history,
            history_path,
            path,
            frame: None,
            log_id: 1,
            buf,
            data: None,
            is_bin: false,
        }
    }

    /// Writes a frame entry to the history file.
    ///
    /// Entries use `id | role.as_arrow() | size`, with a trailing `| b`
    /// for binary frames.
    ///
    /// # Steps
    ///
    /// 1. Get the size and binary flag. `Server` data comes from
    ///    `self.data`; `Client` data comes from `self.frame`.
    /// 2. Build `id | as_arrow(role) | size [| b]`.
    ///
    /// # Errors
    ///
    /// `FileErrorInfo`.
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
                FileErrorInfo::write_fail(&self.history_path, e)
            })
    }

    pub fn add_frame(&mut self, frame: Message) {
        self.frame = Some(frame);
    }

    pub fn set_is_bin(&mut self, is_bin: bool) {
        self.is_bin = is_bin;
    }

    pub async fn rewind_scratch(&mut self) -> Result<(), FileErrorInfo> {
        self.file
            .seek(SeekFrom::Start(0))
            .await
            .map_err(|e| FileErrorInfo::seek_fail(&self.scratch_path, e))?;
        Ok(())
    }
}
