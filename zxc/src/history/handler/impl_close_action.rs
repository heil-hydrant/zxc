use std::io::{Error, ErrorKind, IoSlice};

use header_plz::abnf::LF;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::core::HISTORY_STATE_FILE;
use crate::history::HistoryHandler;
use crate::run::boundary::CloseAction;

/* Steps:
 *      1. If the storage is not empty, create and open file .history.state
 *         file
 *
 *      2. Convert storage to IoSlice and write to state file
 */

impl CloseAction for HistoryHandler {
    async fn close_action(&mut self) -> Result<(), Error> {
        if !self.ui_storage.is_empty() {
            let mut file = File::create_new(HISTORY_STATE_FILE).await?;
            write_history_state(&mut file, &self.ui_storage).await?;
        }
        Ok(())
    }
}

/* Steps:
 *      1. Convert storage to IoSlice
 *
 *      2. Loop write_vectored until every slice is written
 *
 * Errors:
 *      ErrorKind::WriteZero
 */
async fn write_history_state(
    file: &mut File,
    storage: &[String],
) -> Result<(), Error> {
    let mut io_slices: Vec<IoSlice<'_>> =
        Vec::with_capacity(storage.len() * 2);

    storage.iter().for_each(|entry| {
        io_slices.push(IoSlice::new(entry.as_bytes()));
        io_slices.push(IoSlice::new(&[LF]));
    });

    let mut slices: &mut [IoSlice<'_>] = &mut io_slices;
    while !slices.is_empty() {
        let n = file.write_vectored(slices).await?;
        if n == 0 {
            return Err(Error::new(
                ErrorKind::WriteZero,
                "history state write zero",
            ));
        }
        IoSlice::advance_slices(&mut slices, n);
    }
    file.flush().await
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[tokio::test]
    async fn test_write_history_state_writes_every_byte() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "zxc_history_state_{}_{}",
            std::process::id(),
            nanos
        ));
        let mut file = File::create_new(&path).await.unwrap();

        let entry = "a".repeat(1024 * 1024);
        let storage = vec![entry.clone(), entry.clone(), entry];

        write_history_state(&mut file, &storage)
            .await
            .unwrap();
        drop(file);

        let written = std::fs::read(&path).unwrap();
        let mut expected =
            Vec::with_capacity(storage.len() * (1024 * 1024 + 1));
        for e in &storage {
            expected.extend_from_slice(e.as_bytes());
            expected.push(LF);
        }
        assert_eq!(written, expected);

        std::fs::remove_file(&path).unwrap();
    }
}
