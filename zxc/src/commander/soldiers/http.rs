use rustc_hash::FxHashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::commander::CommanderResponse;

// Stores map of soldier communication channels
#[derive(Default)]
pub struct HttpCommStorage {
    store: FxHashMap<usize, Sender<CommanderResponse>>,
}

impl HttpCommStorage {
    pub fn add_handle(&mut self, id: usize) -> Receiver<CommanderResponse> {
        let (tx, rx) = mpsc::channel::<CommanderResponse>(1);
        self.store.insert(id, tx);
        rx
    }

    pub fn pop_sender(
        &mut self,
        id: usize,
    ) -> Option<Sender<CommanderResponse>> {
        self.store.remove(&id)
    }

    pub fn remove(&mut self, id: usize) -> bool {
        self.store.remove(&id).is_some()
    }

    pub fn get(&self, id: usize) -> Option<&Sender<CommanderResponse>> {
        self.store.get(&id)
    }
}
