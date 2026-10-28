use bytes::BytesMut;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Receiver;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol;

use super::Roneone;
use crate::CAPACITY_2MB;
use crate::file_types::EXT_WRES;
use crate::history::wshistory::HISTORY_WS_WSESS;
use crate::repeater::prelude::*;

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

/* Description:
 *      Async conversion from Roneone -> RWebSocket
 *
 * Args:
 *      mut self
 *      receiver: Receiver<RepeaterWsMessage>
 *
 * Steps:
 *      1. Build Scratch file, changedir/rep.res to dir/scratch.wreq, open file
 *      2. Build History file, dir/ws.wsess and Open History file
 *      3. Set path to dir/0.wres, return to original directory
 *      4. Build new ws stream, WebSocketStream::from_raw_socket()
 *      5. Clear previous Buffer
 *
 * Returns:
 *      Ok(RWebSocket)
 *
 * Error:
 *      - WsCreationError::NoPath [2] [4]
 */

impl<T> ToRepeaterWs<T> for Roneone<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    async fn to_rws(
        mut self,
        receiver: Receiver<RepeaterWsMsg>,
    ) -> Result<RWebSocket<T>, RepeaterError> {
        // 1. change path from dir/rep.res to dir/scratch.wreq
        self.path
            .set_file_name(REPEATER_WS_SCRATCH_FILENAME);

        let scratch_file = File::open(&self.path)
            .await
            .map_err(RepeaterError::ScratchFile)?;

        // 2. build History file
        self.path
            .set_file_name(HISTORY_WS_WSESS);

        let history_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .await
            .map_err(RepeaterError::HistoryFile)?;

        // 3. return to original directory
        // dir/0.wres
        self.path.set_file_name("0");
        self.path.set_extension(EXT_WRES);

        // 4. build new ws stream
        let stream = WebSocketStream::from_raw_socket(
            self.stream,
            protocol::Role::Client,
            None,
        )
        .await;

        // 5. Clear Buffer
        self.buf.clear();

        Ok(RWebSocket::new(
            stream,
            receiver,
            self.path,
            scratch_file,
            self.buf,
            history_file,
        ))
    }
}
