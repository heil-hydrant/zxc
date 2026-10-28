use tokio::fs::File;
use tracing::trace;

use crate::interceptor::*;
use crate::io::file::{FileErrorInfo, rewrite_file};
use crate::proxy::prelude::*;

pub trait Rewrite {
    fn file_and_data(&mut self) -> (&mut File, &[u8]);
}

pub async fn rewrite_log<T>(
    mut conn: T,
    resume_info: ResumeInfo,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: Rewrite,
{
    let (file, data) = conn.file_and_data();
    rewrite_file(file, data)
        .await
        .map_err(|(event, e)| FileErrorInfo::from((file, event, e)))?;
    if let Some(info) = resume_info.into_server_info() {
        trace!("new connection");
        return Ok(ProxyState::NewConnection(conn, info));
    }

    trace!("send");
    Ok(ProxyState::Send(conn))
}
