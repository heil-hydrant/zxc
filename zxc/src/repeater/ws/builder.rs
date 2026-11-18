use std::path::PathBuf;

use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::trace;

use crate::file_types::EXT_WREQ;
use crate::forward_info::ForwardInfo;
use crate::history::HISTORY_WS_HIS;
use crate::io::*;
use crate::proxy::prelude::*;
use crate::proxy::server_info::json::ServerInfoJson;
use crate::repeater::prelude::*;

#[derive(Debug, Error)]
pub enum RWsBuildError {
    #[error(transparent)]
    DirError(#[from] DirError),
    #[error("not enough path components: {0}")]
    NotEnoughComponents(String),
    #[error(transparent)]
    ServerInfoRead(#[from] ServerInfoReadError),
    #[error("invalid repeater id: {0}")]
    IdNotValid(String),
    #[error("request file copy: {0}")]
    CopyFile(#[from] std::io::Error),
}

/// Builds a WebSocket repeater session.
///
/// # Steps
///
/// 1. Build the destination repeater file
///    `./history/1/websocket/r-ws-1/scratch.wreq`.
/// 2. Get `$id` from the path, `./history/$id`.
/// 3. Get the `ServerInfoJson` for the ID from `ws.whis`.
/// 4. Build the source HTTP request file `./history/$id/$id.req`.
/// 5. Build the destination HTTP request file
///    `./history/1/websocket/r-ws-1/rep.req`.
/// 6. Copy the source file to the destination.
/// 7. Set `info.server_info` to the `ServerInfoJson`.
pub async fn build_repeater_dest_ws(
    info: &mut ForwardInfo,
) -> Result<PathBuf, RWsBuildError> {
    let dst = build_repeater_dest(&info.file, EXT_WREQ).await?;
    let id = info
        .file
        .components()
        .nth(2)
        .ok_or_else(|| {
            RWsBuildError::NotEnoughComponents(
                dst.to_string_lossy().to_string(),
            )
        })?
        .as_os_str()
        .to_str()
        .ok_or_else(|| {
            RWsBuildError::IdNotValid(dst.to_string_lossy().to_string())
        })?;
    let server_info = read_server_info_from_whis(id).await?;
    let src_req_file = format!("./history/{0}/{0}.req", id);
    let mut dst_req_file = dst.clone();
    dst_req_file.pop();
    dst_req_file.push(REPEATER_HTTP_FILENAME_REQ);
    copy_with_request_metadata(&src_req_file, &dst_req_file).await?;
    info.server_info = Some(server_info);
    Ok(dst)
}

#[derive(Debug, Error)]
pub enum ServerInfoReadError {
    #[error("websocket history file read: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("no host for websocket id {0}")]
    NoHost(String),
    #[error("invalid websocket scheme: {0}")]
    NotValidScheme(String),
}

/// Reads the server info for `id` from `ws.whis`.
///
/// Lines use `id | Scheme | Host`.
///
/// # Steps
///
/// 1. Read `ws.whis`.
/// 2. Find the line with the given ID.
/// 3. Get the scheme and host from the line.
/// 4. Build `ServerInfoJson` from the scheme and host.
pub async fn read_server_info_from_whis(
    id: &str,
) -> Result<ServerInfoJson, ServerInfoReadError> {
    let file = File::open(HISTORY_WS_HIS).await?;
    let bufreader = BufReader::new(file);
    let mut lines = bufreader.lines();
    while let Some(line) = lines.next_line().await? {
        if let (Some(scheme), Some(host)) = parse_scheme_host(&line, id)? {
            let scheme_enum = Scheme::try_from(scheme)
                .map_err(ServerInfoReadError::NotValidScheme)?;
            let server_info =
                ServerInfoJson::new(host.to_string(), scheme_enum, None);
            trace!("server info| {}| {:?}", id, server_info);
            return Ok(server_info);
        }
    }
    Err(ServerInfoReadError::NoHost(id.to_string()))
}

/// Parses `Scheme` and `Host` for `id` from `ws.whis` lines.
///
/// Lines use `id | Scheme | Host`.
fn parse_scheme_host<'a>(
    lines: &'a str,
    id: &str,
) -> Result<(Option<&'a str>, Option<&'a str>), std::io::Error> {
    let mut parts = lines.split(" | ");
    if let Some(sid) = parts.next()
        && sid == id
    {
        return Ok((parts.next(), parts.next()));
    }
    Ok((None, None))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn test_parse_scheme_host_whis() -> Result<(), Box<dyn Error>> {
        let lines = "1 | http | 127.0.0.1:9001";
        if let (Some(protocol), Some(address)) = parse_scheme_host(lines, "1")?
        {
            assert_eq!(protocol, "http");
            assert_eq!(address, "127.0.0.1:9001");
        } else {
            panic!("");
        }

        Ok(())
    }
}
