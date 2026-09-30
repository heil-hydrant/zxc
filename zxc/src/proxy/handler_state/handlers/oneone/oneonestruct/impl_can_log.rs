use super::*;
use crate::proxy::handler_state::transition::CanLog;

impl<T, E, U> CanLog for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    #[inline(always)]
    fn can_log(&self) -> bool {
        self.path.is_some()
    }
}
