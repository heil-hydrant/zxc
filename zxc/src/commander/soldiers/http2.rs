use rustc_hash::FxHashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::commander::CommanderResponse;

#[derive(Default)]
pub struct Http2CommStorage {
    conns: FxHashMap<usize, FxHashMap<u32, Sender<CommanderResponse>>>,
}

impl Http2CommStorage {
    pub fn add_stream_handle(
        &mut self,
        conn_id: usize,
        stream_id: u32,
    ) -> Receiver<CommanderResponse> {
        let (tx, rx) = mpsc::channel::<CommanderResponse>(1);
        self.conns
            .entry(conn_id)
            .or_default()
            .insert(stream_id, tx);
        rx
    }

    pub fn get_stream(
        &self,
        conn_id: usize,
        stream_id: u32,
    ) -> Option<&Sender<CommanderResponse>> {
        self.conns
            .get(&conn_id)?
            .get(&stream_id)
    }

    pub fn remove_stream(&mut self, conn_id: usize, stream_id: u32) -> bool {
        if let Some(streams) = self.conns.get_mut(&conn_id) {
            let removed = streams.remove(&stream_id).is_some();
            if streams.is_empty() {
                self.conns.remove(&conn_id);
            }
            removed
        } else {
            false
        }
    }

    pub fn remove_connection(
        &mut self,
        conn_id: usize,
    ) -> Option<FxHashMap<u32, Sender<CommanderResponse>>> {
        self.conns.remove(&conn_id)
    }
}
