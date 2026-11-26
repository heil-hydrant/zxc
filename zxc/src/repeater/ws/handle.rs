use std::path::PathBuf;

use futures_util::{SinkExt, StreamExt};
use header_plz::{OneInfoLine, OneResponseLine};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
    BufReader,
};
use tokio::select;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{Instrument, Level, error, span, trace};

use crate::async_step::AsyncStep;
use crate::file_types::EXT_WREQ;
use crate::forward_info::ForwardInfo;
use crate::history::HISTORY_WS_HIS;
use crate::io::*;
use crate::proxy::prelude::*;
use crate::proxy::server_info::json::ServerInfoJson;
use crate::repeater::prelude::*;

#[derive(Debug, Error)]
pub(crate) enum RWsBuildError {
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
pub async fn build_repeater_dest_ws(
    info: &mut ForwardInfo,
) -> Result<PathBuf, RWsBuildError> {
    // `./history/1/websocket/r-ws-1/scratch.wreq`.
    let dst = build_repeater_dest(&info.file, EXT_WREQ).await?;

    // `./history/$id`.
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

    // build source req file
    let src_req_file = format!("./history/{0}/{0}.req", id);
    let mut dst_req_file = dst.clone();
    dst_req_file.pop();
    dst_req_file.push(REPEATER_HTTP_FILENAME_REQ);
    copy_with_request_metadata(&src_req_file, &dst_req_file).await?;
    info.server_info = Some(server_info);
    Ok(dst)
}

#[derive(Debug, Error)]
pub(crate) enum ServerInfoReadError {
    #[error("websocket history file read: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("no host for websocket id {0}")]
    NoHost(String),
    #[error("invalid websocket scheme: {0}")]
    NotValidScheme(String),
}

/// Reads the server info for `id` from `ws.whis`.
///
/// format: `id | Scheme | Host`.
async fn read_server_info_from_whis(
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
/// format: `id | Scheme | Host`.
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

/// Spawns a WebSocket task for an upgraded repeater connection.
pub async fn spawn_repeater_ws<T>(
    conn: RepeaterConn<T>,
    index: usize,
) -> Result<RepeaterWsHandle, RepeaterError>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin + Send + 'static,
{
    let mut http_conn = handle_http(conn).await?;
    let response = http_conn.get_payload().unwrap();
    let scode = OneResponseLine::try_build_infoline(response)?
        .status()?
        .into();
    match scode {
        101 => {
            let span = span!(Level::INFO, "repeater_ws", index);
            let (tx, rx) = mpsc::channel::<RepeaterWsMsg>(1);
            let rws = http_conn.into_rws(rx).await?;
            let task = tokio::spawn(async move {
                if let Err(e) = repeater_handle_ws(rws)
                    .instrument(span)
                    .await
                {
                    error!("ws handle| {:?}", e);
                }
            });
            let handle = RepeaterWsHandle::new(index, tx, task);
            Ok(handle)
        }
        scode => Err(RepeaterError::WsWrongScode(scode)),
    }
}

/// Handles the WebSocket repeater task.
async fn repeater_handle_ws<T>(
    mut rws: RWebSocket<T>,
) -> Result<(), RepeaterError>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    trace!("ws start");
    loop {
        select! {
            result = rws.stream.next() => {
                match result {
                    Some(frame) => {
                        rws.add_frame(frame.map_err(WsError::Read)?);
                        trace!("from server");
                        rws = log_response(rws).await?;
                        rws.log(Role::Client).await?;
                    }
                    None => {
                        trace!("server closed ws stream");
                        return Ok(());
                    }
                }
            }
            result = rws.receiver.recv() => {
                match result {
                    Some(msg) => {
                        trace!("from client");
                        match msg {
                            RepeaterWsMsg::Read { is_bin } => {
                                rws.set_is_bin(is_bin);
                                let mut state = RepeaterState::ReadFromFile(rws);
                                rws = loop {
                                    state = state.next().await?;
                                    match state {
                                        RepeaterState::Receive(conn) => break conn,
                                        _ => continue
                                    }
                                };
                                rws.log(Role::Server).await?;
                                rws.rewind_scratch().await?;
                            }
                            RepeaterWsMsg::Close => {
                                let close_msg = Message::Close(None);
                                rws.stream.send(close_msg).await.map_err(|_| WsError::Write)?;
                                trace!("cls frame sent");
                                return Ok(());
                            }
                        }
                    }
                    None => {
                        trace!("repeater ui receiver dropped");
                        return Ok(());
                    }
                }
            }
        }
    }
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
