use bytes::BytesMut;

use super::RWebSocket;
use crate::proxy::handler_state::transition::read_modified_file::{
    AddRaw, WS_ADD_RAW_PANIC
};

// Blanket Implementation
impl<T> AddRaw for RWebSocket<T> {
    fn add_raw(&mut self, _: BytesMut) {
        panic!("{}", WS_ADD_RAW_PANIC);
    }
}
