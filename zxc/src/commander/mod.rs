//! Central actor routing messages between the proxy, the UI, and TLS state.
//!
//! It owns the soldier registry, window channels, and global config; every
//! intercepted message receives exactly one response.
pub mod captain_crypto;
pub mod codec;
pub mod communicate;
pub mod error;
mod protocol;
pub mod soldiers;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub use captain_crypto::{
    ALPN_H1, ALPN_H2, Alpn, CaptainCrypto, CertDigest, CertError,
};
pub use codec::{Codec, CodecError, perform_codec_op};
pub use communicate::request::CommanderRequest;
pub use communicate::response::{CommanderResponse, WrongMessage};
pub use communicate::{HistoryComm, InterceptorComm, RepeaterComm};
pub use error::*;
pub use protocol::*;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};

use self::soldiers::Soldiers;
use crate::config::{Config, parse_global_config, parse_local_config};
use crate::file_types::FileType;
use crate::forward_info::{ForwardInfo, Module};
use crate::history::{CommanderToHistory, HistoryUIOps};
use crate::id::SoldierId;
use crate::interceptor::InterUIOps;

pub struct Commander {
    captain_crypto: CaptainCrypto,
    comm_addon: Sender<ForwardInfo>,
    comm_history: HistoryComm,
    comm_interceptor: InterceptorComm,
    comm_repeater: RepeaterComm,
    comm_soldiers: Receiver<CommanderRequest>,
    config: Option<Config>,
    soldiers: Soldiers,
}

impl Commander {
    #[inline(always)]
    pub fn new(
        captain_crypto: CaptainCrypto,
        comm_addon: Sender<ForwardInfo>,
        comm_history: HistoryComm,
        comm_interceptor: InterceptorComm,
        comm_repeater: RepeaterComm,
        comm_soldiers: Receiver<CommanderRequest>,
        config: Option<Config>,
    ) -> Self {
        Self {
            captain_crypto,
            comm_addon,
            comm_history,
            comm_interceptor,
            comm_repeater,
            comm_soldiers,
            config,
            soldiers: Soldiers::default(),
        }
    }

    async fn handle_soldier(
        &mut self,
        request: CommanderRequest,
    ) -> Result<(), CommunicateError> {
        match request {
            CommanderRequest::ShouldProxy(id, host, tx) => {
                let should_proxy = self
                    .config
                    .as_ref()
                    .is_none_or(|config| config.should_proxy(&host));
                let tosend = if should_proxy {
                    trace!("proxying| {host}");
                    Some(self.soldiers.add_http_handle(id))
                } else {
                    trace!("relay| {host}");
                    None
                };
                tx.send(tosend)
                    .map_err(|_| CommunicateError::ShouldProxy)
            }

            CommanderRequest::H2StreamRegister(id, tx) => {
                trace!("h2 stream registered | {id}");
                let rx = match id {
                    SoldierId::H2Stream {
                        conn_id,
                        stream_id,
                    } => self
                        .soldiers
                        .add_h2_stream_handle(conn_id, stream_id),
                    SoldierId::Connection(conn_id) => {
                        self.soldiers.add_http_handle(conn_id)
                    }
                };
                tx.send(rx)
                    .map_err(|_| CommunicateError::H2StreamRegister)
            }

            CommanderRequest::WsLog(id, role) => {
                trace!("wslog| {}| {}", id, role);
                self.soldiers
                    .send_ws_log(id, role)
                    .await
            }

            CommanderRequest::ShouldInterceptWsRespone(id) => {
                trace!("should intercept ws response");
                self.soldiers
                    .ws_send_should_intercept_response(id)
                    .await
            }

            CommanderRequest::Intercept(id, info) => {
                if !self.comm_interceptor.status() {
                    trace!("interceptor off");
                    let response = CommanderResponse::Resume(None);
                    let ft = info.file_type();
                    self.soldiers
                        .send_response_ft(id, ft, response)
                        .await?;
                } else {
                    self.comm_interceptor
                        .send_to_interceptor(id, info)
                        .await
                        .map_err(|_| CommunicateError::InterceptorSend)?;
                }
                Ok(())
            }

            CommanderRequest::WsRegister(id) => {
                let conn_id = id.conn_id();
                let server_sender = self.soldiers.pop_http_sender(conn_id)?;

                let (client_sender, client_receiver) =
                    channel::<CommanderResponse>(1);

                let tosend =
                    (client_receiver, self.comm_history.to_history.clone());
                let resp = CommanderResponse::WsRegisterReply(tosend);
                server_sender.send(resp).await?;
                self.soldiers
                    .add_ws_handle(id, client_sender, server_sender);
                trace!("ws registered| {}", id);
                Ok(())
            }

            CommanderRequest::Close(id) => {
                self.comm_interceptor.purge_soldier(id);
                if self.soldiers.remove_from_http_store(id) {
                    trace!("removed| http| {}", id);
                } else if self.soldiers.remove_from_ws_store(id) {
                    trace!("removed| ws| {}", id);
                    let msg = CommanderToHistory::RemoveWs(id);
                    self.comm_history
                        .to_history
                        .send(msg)
                        .await?;
                }
                Ok(())
            }
            _ => {
                let (id, response) = match request {
                    CommanderRequest::GetClientConnector(id) => {
                        let connector = self.captain_crypto.get_connector();
                        (
                            SoldierId::Connection(id),
                            CommanderResponse::ClientConfig(connector),
                        )
                    }

                    CommanderRequest::GetVerifier(id) => {
                        let verifier = self.captain_crypto.get_verifier();
                        (
                            SoldierId::Connection(id),
                            CommanderResponse::Verifier(verifier),
                        )
                    }
                    CommanderRequest::CheckServerConfigCache(
                        id,
                        verified,
                        digest,
                        alpn,
                    ) => {
                        let config = self
                            .captain_crypto
                            .check_server_config_cache(verified, digest, alpn);
                        if config.is_some() {
                            trace!("cert exists| Y");
                        } else {
                            trace!("cert exists| N");
                        }
                        let response = CommanderResponse::ServerConfig(config);
                        (SoldierId::Connection(id), response)
                    }

                    CommanderRequest::GetServerConfig(
                        id,
                        verified,
                        digest,
                        cert,
                        alpn,
                    ) => {
                        let result = self
                            .captain_crypto
                            .get_server_config(verified, digest, cert, alpn);
                        let response =
                            CommanderResponse::NewCertificate(result);
                        (SoldierId::Connection(id), response)
                    }

                    CommanderRequest::ShouldLogHttp(id, ext) => {
                        let result = self
                            .config
                            .as_ref()
                            .is_none_or(|config| config.should_log(ext));
                        trace!("http should log| {}| {}", id, result);
                        let tosend = if result {
                            self.get_http_log_path(id).await
                        } else {
                            None
                        };

                        let response = CommanderResponse::HttpLog(tosend);
                        (id, response)
                    }

                    CommanderRequest::ShouldLogHttpCt(id, ct) => {
                        let result =
                            self.config
                                .as_ref()
                                .is_none_or(|config| {
                                    !config.in_excluded_content_types(ct)
                                });
                        trace!(
                            "http should log ct| {}| {} | {}",
                            id, ct, result
                        );
                        let tosend = if result {
                            self.get_http_log_path(id).await
                        } else {
                            None
                        };
                        let response = CommanderResponse::HttpLog(tosend);
                        (id, response)
                    }

                    CommanderRequest::ShouldProxyWs(id) => {
                        let result = self
                            .config
                            .as_ref()
                            .is_some_and(|config| config.with_ws());
                        let response = CommanderResponse::WsProxyReply(result);
                        (id, response)
                    }
                    _ => unreachable!(""),
                };
                self.soldiers
                    .send_response_ft(id, &FileType::Req, response)
                    .await
            }
        }
    }

    async fn get_http_log_path(
        &mut self,
        id: SoldierId,
    ) -> Option<(usize, PathBuf, Sender<CommanderToHistory>)> {
        let (index, path) = create_http_log_dir(
            self.comm_history.path(),
            self.comm_history.http_log_index,
            MAX_HTTP_LOG_INDEX_ATTEMPTS,
        )
        .await?;
        trace!("log path| {}| {}", id, path.to_string_lossy());
        self.comm_history.http_log_index = index + 1;
        Some((index, path, self.comm_history.to_history.clone()))
    }

    async fn empty_resume_queue(&mut self) {
        self.soldiers
            .broadcast_none_http(
                self.comm_interceptor
                    .http_queue_as_mut(),
            )
            .await;
        trace!("http queue emptied");

        self.soldiers
            .broadcast_none_wreq(
                self.comm_interceptor
                    .wreq_queue_as_mut(),
            )
            .await;
        trace!("wreq queue emptied");

        self.soldiers
            .broadcast_none_wres(
                self.comm_interceptor
                    .wres_queue_as_mut(),
            )
            .await;
        trace!("wres queue emptied");
    }

    async fn handle_interceptor(
        &mut self,
        msg: InterUIOps,
    ) -> Result<(), CommunicateError> {
        match msg {
            InterUIOps::Toggle => {
                trace!("toggle");
                self.comm_interceptor.toggle();
                if !self.comm_interceptor.status() {
                    self.empty_resume_queue().await;
                }
                Ok(())
            }
            InterUIOps::Forward(finfo) => {
                trace!("interceptor forward");
                self.forward(finfo).await
            }
            _ => {
                let (log_id, ft, response) = match msg {
                    InterUIOps::Resume(resume_info) => {
                        trace!("resume");
                        (
                            resume_info.id,
                            resume_info.file_type(),
                            CommanderResponse::Resume(Some(resume_info)),
                        )
                    }
                    InterUIOps::Drop(id, ft) => {
                        trace!("drop");
                        (id, ft, CommanderResponse::Drop)
                    }
                    _ => unreachable!(),
                };
                let soldier_id = self
                    .comm_interceptor
                    .soldier_id_from_log_id(log_id, &ft)?;

                if response.wreq_need_response() {
                    self.soldiers
                        .set_ws_need_response(soldier_id)?;
                }
                self.soldiers
                    .send_response_ft(soldier_id, &ft, response)
                    .await
            }
        }
    }

    async fn handle_history(
        &mut self,
        msg: HistoryUIOps,
    ) -> Result<(), CommunicateError> {
        match msg {
            HistoryUIOps::ReloadConfig => {
                trace!("reloading config");
                let global_config = parse_global_config()?;
                let local_config = parse_local_config()
                    .map_err(|e| error!("parse local conf| {}", e))
                    .ok();
                self.config = Config::build(local_config, global_config);
            }
            HistoryUIOps::Forward(finfo) => {
                trace!("history forward");
                self.forward(finfo).await?
            }
            msg => error!("Should not be reached| {:?}", msg),
        }
        Ok(())
    }

    pub async fn forward(
        &mut self,
        finfo: ForwardInfo,
    ) -> Result<(), CommunicateError> {
        match finfo.to_module() {
            Module::Repeater => {
                self.comm_repeater
                    .to_repeater
                    .send(finfo)
                    .await
            }
            Module::Addon(_) => self.comm_addon.send(finfo).await,
        }?;
        Ok(())
    }
}

const MAX_HTTP_LOG_INDEX_ATTEMPTS: usize = 1024;

/// Creates an incrementing HTTP log directory under `base`.
async fn create_http_log_dir(
    base: &Path,
    start_index: usize,
    max_attempts: usize,
) -> Option<(usize, PathBuf)> {
    let mut index = start_index;
    let mut path = base.to_path_buf();
    for _ in 0..max_attempts {
        path.push(index.to_string());
        match tokio::fs::create_dir(&path).await {
            Ok(()) => return Some((index, path)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                trace!(
                    "history dir exists| {}| {}",
                    index,
                    path.to_string_lossy()
                );
                path.pop();
                index += 1;
            }
            Err(e) => {
                error!("creating directory| {:?}", e);
                return None;
            }
        }
    }
    error!(
        "creating directory| no free index from {} in {} attempts",
        start_index, max_attempts
    );
    None
}

pub async fn run_commander(mut comm: Commander, token: CancellationToken) {
    debug!("[+] commander started");
    loop {
        tokio::select! {
            biased;
            _ = token.cancelled() => {
                trace!("cancelled");
                return;
            }
            soldier_msg = comm.comm_soldiers.recv() => {
                let Some(smsg) = soldier_msg else {
                    trace!("soldiers channel closed");
                    return;
                };
                if let Err(e) = comm.handle_soldier(smsg).await {
                    error!("soldier| {:?}", e);
                }
            }
            interceptor_msg = comm.comm_interceptor.from_interceptor.recv() => {
                let Some(imsg) = interceptor_msg else {
                    trace!("interceptor channel closed");
                    return;
                };
                if let Err(e) = comm.handle_interceptor(imsg).await {
                    error!("interceptor| {:?}", e);
                }
            }
            history_msg = comm.comm_history.from_history.recv() => {
                let Some(msg) = history_msg else {
                    trace!("history channel closed");
                    return;
                };
                if let Err(e) = comm.handle_history(msg).await {
                    error!("history| {:?}", e);
                }
            }
            repeater_msg = comm.comm_repeater.from_repeater.recv() => {
                let Some(msg) = repeater_msg else {
                    trace!("repeater channel closed");
                    return;
                };
                if let Err(e) = comm.forward(msg).await {
                    error!("repeater| {:?}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "zxc_http_log_{}_{}_{}",
            name,
            std::process::id(),
            nanos
        ))
    }

    #[tokio::test]
    async fn test_create_http_log_dir_uses_start_index() {
        let base = unique_temp_dir("start");
        std::fs::create_dir_all(&base).unwrap();

        let (index, path) = create_http_log_dir(&base, 3, 8)
            .await
            .unwrap();

        assert_eq!(index, 3);
        assert_eq!(path, base.join("3"));
        assert!(path.is_dir());

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[tokio::test]
    async fn test_create_http_log_dir_skips_existing() {
        let base = unique_temp_dir("existing");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir(base.join("7")).unwrap();
        std::fs::create_dir(base.join("8")).unwrap();

        let (index, path) = create_http_log_dir(&base, 7, 8)
            .await
            .unwrap();

        assert_eq!(index, 9);
        assert_eq!(path, base.join("9"));
        assert!(path.is_dir());

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[tokio::test]
    async fn test_create_http_log_dir_stops_at_attempt_limit() {
        let base = unique_temp_dir("limit");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir(base.join("1")).unwrap();
        std::fs::create_dir(base.join("2")).unwrap();

        let result = create_http_log_dir(&base, 1, 2).await;

        assert!(result.is_none());
        assert!(!base.join("3").exists());

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[tokio::test]
    async fn test_create_http_log_dir_returns_none_on_other_error() {
        let base = unique_temp_dir("missing");

        let result = create_http_log_dir(&base, 1, 8).await;

        assert!(result.is_none());
        assert!(!base.exists());
    }
}
