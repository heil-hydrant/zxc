use rustc_hash::FxHashMap;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::commander::CommunicateError;
use crate::file_types::FileType;
use crate::id::SoldierId;
use crate::interceptor::{InterToUI, InterUIOps};

// Handles Commander to interceptor communication
pub struct InterceptorComm {
    pub from_interceptor: Receiver<InterUIOps>,
    to_interceptor: Sender<InterToUI>,
    http_queue: FxHashMap<usize, SoldierId>,
    wreq_queue: FxHashMap<usize, SoldierId>,
    wres_queue: FxHashMap<usize, SoldierId>,
    status: bool,
}

impl InterceptorComm {
    #[inline(always)]
    pub fn new(
        from_interceptor: Receiver<InterUIOps>,
        to_interceptor: Sender<InterToUI>,
    ) -> Self {
        Self {
            from_interceptor,
            to_interceptor,
            http_queue: FxHashMap::default(),
            wreq_queue: FxHashMap::default(),
            wres_queue: FxHashMap::default(),
            status: false,
        }
    }

    pub fn status(&self) -> bool {
        self.status
    }

    pub fn toggle(&mut self) {
        self.status = !self.status;
    }

    pub async fn send_to_interceptor(
        &mut self,
        soldier_id: SoldierId,
        msg: InterToUI,
    ) -> Result<(), SendError<InterToUI>> {
        let queue = if msg.is_http() {
            &mut self.http_queue
        } else if msg.is_wreq() {
            &mut self.wreq_queue
        } else {
            &mut self.wres_queue
        };
        let log_id = msg.id();
        queue.insert(log_id, soldier_id);
        self.to_interceptor.send(msg).await
    }

    pub fn http_queue_as_mut(&mut self) -> &mut FxHashMap<usize, SoldierId> {
        &mut self.http_queue
    }

    pub fn wreq_queue_as_mut(&mut self) -> &mut FxHashMap<usize, SoldierId> {
        &mut self.wreq_queue
    }

    pub fn wres_queue_as_mut(&mut self) -> &mut FxHashMap<usize, SoldierId> {
        &mut self.wres_queue
    }

    pub fn soldier_id_from_log_id(
        &mut self,
        log_id: usize,
        ft: &FileType,
    ) -> Result<SoldierId, CommunicateError> {
        let queue = match ft {
            FileType::Req | FileType::Res => &mut self.http_queue,
            FileType::Wreq => &mut self.wreq_queue,
            FileType::Wres => &mut self.wres_queue,
        };
        queue
            .remove(&log_id)
            .ok_or(CommunicateError::NoId(
                SoldierId::Connection(log_id),
                "resume_intercept",
            ))
    }

    pub fn purge_connection(&mut self, conn_id: usize) {
        self.http_queue
            .retain(|_, sid| sid.conn_id() != conn_id);
        self.wreq_queue
            .retain(|_, sid| sid.conn_id() != conn_id);
        self.wres_queue
            .retain(|_, sid| sid.conn_id() != conn_id);
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::channel;

    use super::*;

    #[tokio::test]
    async fn test_interceptor_comm_hashmap_lookup_and_purge() {
        let (_from_tx, from_rx) = channel(1);
        let (to_tx, mut to_rx) = channel(10);
        let mut comm = InterceptorComm::new(from_rx, to_tx);

        let sid1 = SoldierId::Connection(1);
        let sid2 = SoldierId::H2Stream {
            conn_id: 2,
            stream_id: 3,
        };
        let sid3 = SoldierId::H2Stream {
            conn_id: 2,
            stream_id: 5,
        };

        let msg1 = InterToUI::build_http_res(101);
        let msg2 = InterToUI::build_http_res(102);
        let msg3 = InterToUI::build_http_res(103);

        comm.send_to_interceptor(sid1, msg1)
            .await
            .unwrap();
        comm.send_to_interceptor(sid2, msg2)
            .await
            .unwrap();
        comm.send_to_interceptor(sid3, msg3)
            .await
            .unwrap();

        assert_eq!(comm.http_queue.len(), 3);
        assert_eq!(to_rx.recv().await.unwrap().id(), 101);
        assert_eq!(to_rx.recv().await.unwrap().id(), 102);
        assert_eq!(to_rx.recv().await.unwrap().id(), 103);

        let resolved = comm
            .soldier_id_from_log_id(101, &FileType::Req)
            .unwrap();
        assert_eq!(resolved, sid1);
        assert_eq!(comm.http_queue.len(), 2);

        comm.purge_connection(2);
        assert_eq!(comm.http_queue.len(), 0);
    }
}
