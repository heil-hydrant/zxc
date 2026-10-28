use super::CanCommunicate;
use crate::commander::*;
use crate::interceptor::*;
use crate::proxy::prelude::*;

// Trait to intercept the http/ws request/response.
pub trait Intercept {
    fn get_inter_info(&self) -> InterToUI;
}

pub async fn intercept<T>(
    mut conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: Intercept + CanCommunicate + Id,
{
    let info = conn.get_inter_info();
    let req = CommanderRequest::Intercept(conn.id(), info);
    conn.sender().send(req).await?;
    Ok(ProxyState::ResumeIntercept(conn))
}
