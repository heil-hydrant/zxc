use std::path::PathBuf;

use tokio_tungstenite::tungstenite::Message;

use super::WsStruct;
use crate::proxy::handler_state::transition::{Log, UpdateLogExt};

impl<T, E> Log for WsStruct<T, E> {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn log_data(&self) -> &[u8] {
        match self.frame
            .as_ref()
            .unwrap() // safe to unwrap
            {
                Message::Text(data) => data.as_bytes(),
                Message::Binary(vec) => vec,
                _ => unreachable!()
            }
    }
}

impl<T, E> UpdateLogExt for WsStruct<T, E> {
    fn update_extension(&mut self) {
        self.path
            .set_file_name(self.log_id.to_string());
        self.path
            .set_extension(self.role.ws_ext());
    }
}
