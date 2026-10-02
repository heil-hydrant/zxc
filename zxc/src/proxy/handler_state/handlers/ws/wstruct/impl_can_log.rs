use super::*;
use crate::proxy::handler_state::transition::CanLog;

// Always returns false.
// Each request/response needs to have unique path.
// $id.wreq/wres
impl<T, E> CanLog for WsStruct<T, E> {
    #[inline(always)]
    fn can_log(&self) -> bool {
        false
    }
}
