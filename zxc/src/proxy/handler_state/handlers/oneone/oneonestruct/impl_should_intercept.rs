use super::*;
use crate::proxy::handler_state::QueryCommanderShouldIntercept;
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::handler_state::transition::should_intercept::ShouldIntercept;

/* Steps:
 *      For request, always true.
 *
 *      For response, need_response value by user in resume_info
 */

impl<T, E> ShouldIntercept for OneOneStruct<T, E, OneRequestLine> {
    #[inline(always)]
    fn should_intercept(&self) -> Option<bool> {
        Some(true)
    }
}

impl<T, E> ShouldIntercept for OneOneStruct<T, E, OneResponseLine> {
    #[inline(always)]
    fn should_intercept(&self) -> Option<bool> {
        Some(self.need_response)
    }
}

// Blanket implementation, always panic
impl<T, E, U> QueryCommanderShouldIntercept for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    async fn query_commander_should_intercept(
        &mut self,
    ) -> Result<bool, ProxyStateError> {
        panic!("No QueryCommanderShouldIntercept for OneOneStruct");
    }
}
