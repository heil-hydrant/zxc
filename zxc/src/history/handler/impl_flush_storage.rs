use std::io::IoSlice;

use tokio::net::UnixStream;

use crate::history::HistoryHandler;
use crate::io::{
    CLOSE_SQUARE_BRACKET, DEFAULT_ID, UnixSockError, unix_write_vec_and_flush
};
use crate::run::boundary::FlushStorage;

/* Steps:
 *      1. Build Vec<IoSlice> with capacity of storage length * 3
 *
 *      2. For each entry in storage push
 *          - DEFAULT_ID
 *          - entry
 *          - ']'
 *
 *      3. write to socket
 *
 * Errors:
 *      UnixSockError::Write
 */
impl FlushStorage for HistoryHandler {
    async fn flush_storage(
        &mut self,
        stream: &mut UnixStream,
    ) -> Result<(), UnixSockError> {
        let mut send: Vec<IoSlice<'_>> =
            Vec::with_capacity(self.ui_storage.len() * 3);

        self.ui_storage.iter().for_each(|data| {
            send.push(IoSlice::new(DEFAULT_ID.as_bytes()));
            send.push(IoSlice::new(data.as_bytes()));
            send.push(IoSlice::new(CLOSE_SQUARE_BRACKET.as_bytes()));
        });

        let _ = unix_write_vec_and_flush(stream, &mut send)
            .await
            .map_err(UnixSockError::Write)?;
        self.ui_storage.clear();
        Ok(())
    }
}
