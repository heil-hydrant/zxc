use super::*;
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::handler_state::transition::drop_msg::DropMsg;

// Http/1.1 always close connection
impl<T, E, U> DropMsg for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    #[inline(always)]
    fn continue_on_drop() -> Result<(), ProxyStateError> {
        Err(ProxyStateError::Drop)
    }
}
