use std::path::PathBuf;

use tokio_tungstenite::tungstenite::Message;

use super::WsStruct;
use crate::proxy::handler_state::transition::write_log::log::{Log, NLog};

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

impl<T, E> NLog for WsStruct<T, E> {
    fn log_data(&self) -> impl bytes::Buf {
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
