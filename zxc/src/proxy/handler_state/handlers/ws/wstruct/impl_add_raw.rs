use bytes::BytesMut;

use super::WsStruct;
use crate::proxy::handler_state::transition::read_modified_file::{
    AddRaw, WS_ADD_RAW_PANIC,
};

// Blank implementation. No add_raw for ws
impl<T, E> AddRaw for WsStruct<T, E> {
    #[inline(always)]
    fn add_raw(&mut self, _: BytesMut) {
        panic!("{}", WS_ADD_RAW_PANIC);
    }
}
