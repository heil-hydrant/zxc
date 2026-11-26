use tracing::trace;

use crate::proxy::prelude::*;

const SHOULD_INTERCEPT_UNREACHABLE: &str =
    "QueryCommanderShouldIntercept only applicable for WebSocket";

// Trait to check if request/response needs to be intercepted
// for request, always true
// for response, based on need_response from ui
pub trait ShouldIntercept {
    #[inline(always)]
    fn should_intercept(&self) -> Option<bool> {
        Some(true)
    }

    // method implemented only for ws response
    async fn query_commander_should_intercept(
        &mut self,
    ) -> Result<bool, ProxyStateError> {
        unreachable!("{}", SHOULD_INTERCEPT_UNREACHABLE)
    }
}

pub async fn should_intercept<T>(
    mut conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: ShouldIntercept,
{
    let result = match conn.should_intercept() {
        Some(true) => true,
        None => {
            trace!("should intercept| wres");
            conn.query_commander_should_intercept()
                .await?
        }
        _ => false,
    };
    if result {
        trace!("should intercept| Y");
        return Ok(ProxyState::Intercept(conn));
    }
    trace!("should intercept| N");
    Ok(ProxyState::Send(conn))
}
