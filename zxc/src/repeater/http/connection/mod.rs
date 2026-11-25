use std::path::PathBuf;

use bytes::BytesMut;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Receiver;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol;
use zxc_derive::FileBuf;

use crate::CAPACITY_2MB;
use crate::file_types::EXT_WRES;
use crate::repeater::prelude::*;

mod pipeline;

// repeater http handler struct
#[derive(FileBuf)]
pub struct Roneone<T> {
    buf: BytesMut,
    path: PathBuf,
    stream: T,
    file: File,
    payload: Option<BytesMut>,
    update: bool,
}

impl<T> Roneone<T> {
    pub fn get_payload(&mut self) -> Option<BytesMut> {
        self.payload.take()
    }
}

// Convert (RepeaterConn<T>, File) -> Roneone
impl<T> From<(RepeaterConn<T>, File)> for Roneone<T> {
    fn from((conn, file): (RepeaterConn<T>, File)) -> Self {
        Self {
            buf: BytesMut::with_capacity(CAPACITY_2MB),
            path: conn.path,
            stream: conn.stream,
            file,
            payload: None,
            update: conn.update,
        }
    }
}

// Converts `Roneone` into `RWebSocket`.
impl<T> ToRepeaterWs<T> for Roneone<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    async fn to_rws(
        mut self,
        receiver: Receiver<RepeaterWsMsg>,
    ) -> Result<RWebSocket<T>, RepeaterError> {
        // build scratch path dir/scratch.wreq
        let scratch_path = self
            .path
            .with_file_name(REPEATER_WS_SCRATCH_FILENAME);

        let scratch_file = File::open(&scratch_path)
            .await
            .map_err(RepeaterError::ScratchFile)?;

        // build History file
        let history_path = self
            .path
            .with_file_name(crate::history::HISTORY_WS_WSESS);

        let history_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&history_path)
            .await
            .map_err(RepeaterError::HistoryFile)?;

        // set the frame path to dir/0.wres
        self.path.set_file_name("0");
        self.path.set_extension(EXT_WRES);

        // build new ws stream
        let stream = WebSocketStream::from_raw_socket(
            self.stream,
            protocol::Role::Client,
            None,
        )
        .await;

        // Clear Buffer
        self.buf.clear();

        Ok(RWebSocket::new(
            stream,
            receiver,
            self.path,
            scratch_path,
            scratch_file,
            self.buf,
            history_path,
            history_file,
        ))
    }
}
