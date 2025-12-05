use super::WsStruct;
use crate::proxy::handler_state::transition::write_history::{
    GetHistory, HistoryEnum, WsHistory
};

impl<T, E> GetHistory for WsStruct<T, E> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let ws_his = WsHistory::new(
            self.log_id,
            &self.role,
            self.frame.as_ref().unwrap().is_binary(),
            self.frame.as_ref().unwrap().len(), // safe to unwrap
        );

        HistoryEnum::WebSocket(self.id, ws_his)
    }
}
