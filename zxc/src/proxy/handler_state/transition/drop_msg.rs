use crate::proxy::prelude::*;

// Trait to decide to continue on drop when the message is dropped by
// interceptor.
//
// HTTP connections close because their framing state cannot be reused after
// a dropped message. WebSocket overrides this to keep receiving frames.
pub trait DropMsg {
    #[inline(always)]
    fn continue_on_drop() -> Result<(), ProxyStateError> {
        Err(ProxyStateError::Drop)
    }
}
