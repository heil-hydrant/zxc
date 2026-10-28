use std::time::Duration;

use tokio::select;
use tokio::time::{Instant, interval_at};
use tracing::trace;

use crate::history::HistoryHandler;
use crate::run::boundary::FromCommander;

const LIMIT: usize = 100;

/* Description:
 *      Time based buffer writer.
 *      Writes every 2 seconds.
 *
 * Associcated type:
 *      Number of messages received
 *
 * Steps:
 *      1. Set Interval for 2 seconds.
 *      ----- loop -----
 *          ----- select -----
 *          1. recv_many messages from commander
 *              a. If current Interval is not 2 seconds, set Interval to 2
 *              seconds and reset immediately.
 *              b. Else, wait for timer to tick.
 *          2. If Interval ticked,
 *              a. If there are no messages, increment Interval by 2 seconds
 *              b. If there are messages, return number of messages
 */

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
