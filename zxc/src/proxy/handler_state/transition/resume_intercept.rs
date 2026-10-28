use tracing::trace;

use super::can_communicate::CanCommunicate;
use crate::interceptor::*;
use crate::proxy::prelude::*;

// Update whether the response needs to be updated (http only)
pub trait UpdateResumeInfo {
    fn update_resume_info(&mut self, resume_info: &ResumeInfo);
}

pub async fn resume_intercept<T>(
    mut conn: T,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: UpdateResumeInfo + CanCommunicate,
{
    let response = conn
        .receiver()
        .recv()
        .await
        .ok_or(ProxyStateError::CommanderResponse("resume_intercept"))?;
    if response.is_drop_msg() {
        return Ok(ProxyState::Drop(conn));
    }
    if let Some(resume_info) = Option::<ResumeInfo>::try_from(response)? {
        conn.update_resume_info(&resume_info);
        if resume_info.modified() {
            trace!("file mod| Y");
            return Ok(ProxyState::ReadModFile(conn, resume_info));
            // http req only
        } else if let Some(serve_info) = resume_info.into_server_info() {
            trace!("new conn");
            return Ok(ProxyState::NewConnection(conn, serve_info));
        };
    }
    trace!("file mod| N| => send");
    Ok(ProxyState::Send(conn))
}
