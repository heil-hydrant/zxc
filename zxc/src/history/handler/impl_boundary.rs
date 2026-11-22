use std::io::{Error, ErrorKind, IoSlice};
use std::time::Duration;

use header_plz::abnf::LF;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::select;
use tokio::time::{interval_at, Instant};
use tracing::trace;

use super::HistoryHandler;
use crate::core::HISTORY_STATE_FILE;
use crate::history::*;
use crate::io::*;
use crate::run::boundary::*;

const LIMIT: usize = 100;

// `writev(2)` rejects more than `IOV_MAX` slices with `EINVAL`
// (1024 on Linux and macOS).
const IOV_MAX: usize = 1024;

// Persists buffered history entries before shutdown.
impl CloseAction for HistoryHandler {
    async fn close_action(&mut self) -> Result<(), Error> {
        if !self.ui_storage.is_empty() {
            let mut file = File::create_new(HISTORY_STATE_FILE).await?;
            write_history_state(&mut file, &self.ui_storage).await?;
        }
        Ok(())
    }
}

/// Writes the buffered history entries to the state file.
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
        let n = file
            .write_vectored(&slices[..slices.len().min(IOV_MAX)])
            .await?;
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

// Flushes buffered history entries to the UI socket.
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

        // Stay within `IOV_MAX` and end each chunk on a whole 3-slice
        // frame so a failed chunk cannot leave a truncated envelope.
        let chunk_size = IOV_MAX - IOV_MAX % 3;
        for chunk in send.chunks_mut(chunk_size) {
            let _ = unix_write_vec_and_flush(stream, chunk)
                .await
                .map_err(UnixSockError::Write)?;
        }
        self.ui_storage.clear();
        Ok(())
    }
}

// Buffers commander messages and releases them on a timer.
impl FromCommander for HistoryHandler {
    type Message = usize;

    async fn recv(&mut self) -> Option<Self::Message> {
        let duration = Duration::from_secs(2);
        let mut curr_duration = duration;
        let mut interval =
            interval_at(Instant::now() + curr_duration, curr_duration);
        loop {
            select! {
                n = self.from_commander.recv_many(&mut self.msg_storage, LIMIT) => {
                    if n == 0 {
                        trace!("commander channel closed");
                        return if self.msg_storage.is_empty() {
                            None
                        } else {
                            Some(self.msg_storage.len())
                        };
                    }
                    if interval.period() != duration {
                        trace!("timer reset");
                        curr_duration = duration;
                        interval = interval_at(Instant::now() + duration, duration);
                        interval.reset_immediately();
                    }
                    if self.msg_storage.len() >= LIMIT {
                        trace!("limit reached: {}", self.msg_storage.len());
                        return Some(self.msg_storage.len());
                    }
                }
                _ = interval.tick() => {
                    if self.msg_storage.is_empty() {
                        curr_duration += duration;
                        interval = interval_at(Instant::now() + curr_duration, curr_duration);
                    } else {
                        trace!("tick: {}", self.msg_storage.len());
                        return Some(self.msg_storage.len());
                    }
                }
            }
        }
    }
}

impl HandleCommander for HistoryHandler {
    type Error = HistoryError;

    async fn handle_commander_no_ui(
        &mut self,
        size: usize,
    ) -> Result<(), Self::Error> {
        self.handle_commander(size, false)
            .await?;
        Ok(())
    }

    async fn handle_commander_ui(
        &mut self,
        size: usize,
    ) -> Result<UiReply, Self::Error> {
        self.handle_commander(size, true).await
    }
}

// Handles one decoded UI message for history.
impl HandleUI for HistoryHandler {
    async fn handle_ui(&mut self) -> Result<UiReply, HistoryError> {
        let msg = serde_json::from_slice::<HistoryUImsg>(&self.buf)?;
        if matches!(msg.operation, HistoryUIOps::Close) {
            return Err(UnixSockError::Closed.into());
        }
        self.to_commander
            .send(msg.into())
            .await?;
        Ok(UiReply::None)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::sync::mpsc::channel;
    use tokio::time::timeout;

    use super::*;
    use crate::history::CommanderToHistory;

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

    #[tokio::test]
    async fn recv_returns_none_when_channel_closed() {
        let (tx, rx) = channel::<CommanderToHistory>(1);
        let (to_commander, _to_commander_rx) = channel::<HistoryUIOps>(1);
        let mut handler = HistoryHandler::new(rx, to_commander, Vec::new());
        drop(tx);

        let result = timeout(Duration::from_secs(1), handler.recv())
            .await
            .expect("recv timed out");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn recv_returns_pending_messages_when_channel_closed() {
        let (tx, rx) = channel::<CommanderToHistory>(1);
        let (to_commander, _to_commander_rx) = channel::<HistoryUIOps>(1);
        let mut handler = HistoryHandler::new(rx, to_commander, Vec::new());
        tx.send(CommanderToHistory::Http("pending".to_string()))
            .await
            .unwrap();
        drop(tx);

        let result = timeout(Duration::from_secs(1), handler.recv())
            .await
            .expect("recv timed out");

        assert_eq!(result, Some(1));
    }
}
