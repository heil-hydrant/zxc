use std::io::{Error, IoSlice};

use header_plz::abnf::LF;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::HistoryHandler;
use crate::builder::HISTORY_STATE_FILE;
use crate::run::boundary::CloseAction;

/* Steps:
 *      1. If the storage is not empty, create and open file .history.state
 *         file
 *
 *      2. Convert storage to IoSlice and write to state file
 *
 *  TODO:
 *      Handle amount the data written
 */

impl CloseAction for HistoryHandler {
    async fn close_action(&mut self) -> Result<(), Error> {
        if !self.ui_storage.is_empty() {
            let mut file = File::create_new(HISTORY_STATE_FILE).await?;
            let mut io_slices: Vec<IoSlice<'_>> =
                Vec::with_capacity(self.ui_storage.len() * 2);

            self.ui_storage
                .iter()
                .for_each(|entry| {
                    io_slices.push(IoSlice::new(entry.as_bytes()));
                    io_slices.push(IoSlice::new(&[LF]));
                });
            let _ = file.write_vectored(&io_slices).await?;
            file.flush().await?;
        }
        Ok(())
    }
}
