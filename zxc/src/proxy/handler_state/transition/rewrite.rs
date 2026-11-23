use std::path::Path;

use tokio::fs::File;
use tracing::trace;

use crate::interceptor::*;
use crate::io::file::rewrite_file;
use crate::proxy::prelude::*;

pub trait Rewrite {
    fn file_data_and_path(&mut self) -> (&mut File, &[u8], &Path);
}

pub async fn rewrite_log<T>(
    mut conn: T,
    resume_info: ResumeInfo,
) -> Result<ProxyState<T>, ProxyStateError>
where
    T: Rewrite,
{
    let (file, data, path) = conn.file_data_and_path();
    rewrite_file(file, data, path).await?;
    if let Some(info) = resume_info.into_server_info() {
        trace!("new connection");
        return Ok(ProxyState::NewConnection(conn, info));
    }

    trace!("send");
    Ok(ProxyState::Send(conn))
}
