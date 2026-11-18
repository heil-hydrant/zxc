use std::time::Duration;

use tokio::select;
use tokio::time::{Instant, interval_at};
use tracing::trace;

use crate::history::HistoryHandler;
use crate::run::boundary::FromCommander;

const LIMIT: usize = 100;

// Buffers commander messages and releases them on a timer.
//
// Yields the number of messages received.
//
// # Steps
//
// 1. Set the interval to 2 seconds.
// 2. Loop over a `select!`:
//    a. Receive messages from the commander with `recv_many`. If the
//       current interval is not 2 seconds, set it to 2 seconds and
//       reset it immediately, otherwise wait for the timer to tick.
//    b. If the interval ticked and the storage is empty, increase the
//       interval by 2 seconds.
//    c. If the interval ticked and there are messages, return the
//       number of messages.
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
                }
                _ = interval.tick() => {
                    trace!("time elapsed | {}", curr_duration.as_secs());
                    if self.msg_storage.is_empty() {
                        // add duration to tick
                        curr_duration += duration;
                        interval = interval_at(Instant::now() + curr_duration, curr_duration);
                    } else {
                        return Some(self.msg_storage.len())
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::sync::mpsc::channel;
    use tokio::time::timeout;

    use super::*;
    use crate::history::{CommanderToHistory, HistoryUIOps};
    use crate::run::boundary::FromCommander;

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
