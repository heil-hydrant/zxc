pub mod captain_crypto;
pub mod codec;
pub mod communicate;
pub mod error;
use communicate::comm_history::HistoryComm;
use communicate::comm_interceptor::InterceptorComm;
use communicate::comm_repeater::RepeaterComm;
use tokio_util::sync::CancellationToken;
mod protocol;
use std::path::PathBuf;

use captain_crypto::*;
pub use communicate::request::CommanderRequest;
pub use communicate::response::CommanderResponse;
use error::*;
pub use protocol::*;
use soldiers::Soldiers;
use tokio::fs::create_dir;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::{debug, error, trace};
pub mod soldiers;

use crate::config::Config;
use crate::config::global::parser::parse_global_config;
use crate::config::local::io::parse_local_config;
use crate::file_types::FileType;
use crate::forward_info::{ForwardInfo, Module};
use crate::history::message::from_commander::CommanderToHistory;
use crate::history::message::from_ui::HistoryUIOps;
use crate::interceptor::message::from_ui::InterUIOps;

const WS_REGISTER: &str = "ws_register";

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

    pub async fn handle_soldier(
        &mut self,
        request: CommanderRequest,
    ) -> Result<(), CommunicateError> {
        match request {
            /* Associated Values:
             *      host        : String
             *      sender      : <Option<Receiver<CommanderResponse>>>
             *
             * Steps:
             *      1. if config is some, call should_proxy() with host
             *      2. If true, create new Receiver<CommanderResponse> for
             *      the connection by calling http_storage.add_handle() with
             *      the id
             *      2. If false, send None
             *      3. send result through oneshot sender
             *
             * Error:
             *      CommunicateError::ShouldProxy
             */
            CommanderRequest::ShouldProxy(id, host, tx) => {
                let tosend = if let Some(config) = self.config.as_ref() {
                    if config.should_proxy(&host) {
                        trace!("proxying| {}", &host);
                        Some(self.soldiers.add_http_handle(id))
                    } else {
                        trace!("relay| {}", &host);
                        None
                    }
                } else {
                    trace!("default proxying| {}", &host);
                    Some(self.soldiers.add_http_handle(id))
                };
                tx.send(tosend)
                    .map_err(|_| CommunicateError::ShouldProxy)
            }

            /* Error:
             *      CommunicateError::NoId
             *      CommunicateError::Send
             */
            CommanderRequest::WsLog(id, role) => {
                trace!("wslog| {}| {}", id, role);
                self.soldiers
                    .send_ws_log(id, role)
                    .await
            }

            /* Error:
             *      CommunicateError::NoId
             *      CommunicateError::Send
             */
            CommanderRequest::ShouldInterceptWsRespone(id) => {
                trace!("should intercept ws response");
                self.soldiers
                    .ws_send_should_intercept_response(id)
                    .await
            }

            /* Associated Values:
             *      info    : InterToUI
             *
             * Steps:
             *      1. If interception is off, send none to respective conn
             *
             *      2. else send msg to ui through interceptor_handle by calling
             *         interceptor_handle.send_to_interceptor()
             *
             * Error:
             *      CommunicateError::NoId
             *      CommunicateError::Send
             *      CommunicateError::InterceptorSend
             */
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

            /* Steps:
             *      1. Pop the Server sender from the http_storage
             *
             *      2. Create mpsc::channel<CommanderResponse> for client.
             *         Server already has a receiver and commander has the
             *         sender.
             *
             *      3. Build new rply with the Receivers and send it through
             *         popped sender
             *
             *      4. Add the Senders to ws_storage by calling add_ws_handle()
             *
             * Error:
             *      CommunicateError::NoId
             */
            CommanderRequest::WsRegister(id) => {
                let server_sender = self.soldiers.pop_http_sender(id)?;

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

            /* Steps:
             *      1. Try to remove from http_storage
             *      2. else remove from ws_storage
             *      3. If removed tom ws_storage, send
             *         CommanderToHistory::RemoveWs to history
             */
            CommanderRequest::Close(id) => {
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
            // Http Related
            // Common Associcated Value:
            //      id: usize
            //      Connection Id
            _ => {
                let (id, response) = match request {
                    CommanderRequest::GetClientConfig(id) => {
                        let connector = self.captain_crypto.get_connector();
                        (id, CommanderResponse::ClientConfig(connector))
                    }

                    CommanderRequest::GetVerifier(id) => {
                        let verifier = self.captain_crypto.get_verifier();
                        (id, CommanderResponse::Verifier(verifier))
                    }

                    /* Associated Values:
                     *      verify_status (vs)      : bool
                     *      digest_to_check (d)     : DigestBytes
                     */
                    CommanderRequest::CheckCertificate(id, vs, d) => {
                        let config = self.captain_crypto.check_serial(vs, d);
                        if config.is_some() {
                            trace!("cert exists| Y");
                        } else {
                            trace!("cert exists| N");
                        }
                        let response = CommanderResponse::ServerConfig(config);
                        (id, response)
                    }

                    /* Associated Values:
                     *      verify_status       : bool
                     *      digest_to_check     : DigestBytes
                     *      certificate         : Vec<CertificateDer<'static>>,
                     *
                     *  Error:
                     *      CommunicateError::GenNewCert
                     */
                    CommanderRequest::GenNewCert(id, vs, d, cert) => {
                        let result = self
                            .captain_crypto
                            .generate_new_cert(vs, d, cert);
                        let response =
                            CommanderResponse::NewCertificate(result);
                        (id, response)
                    }

                    /* Associated Values:
                     *      ext : String
                     *
                     * Steps:
                     *      1. if config is_some call config.should_log() with
                     *         extension, else true
                     *      2. if true, call self.get_http_log_path() with id
                     *         to get new path and Sender<CommanderToHistory>
                     */
                    CommanderRequest::ShouldLogHttp(id, ext) => {
                        let result = self
                            .config
                            .as_ref()
                            .is_none_or(|config| config.should_log(ext)); // 1
                        trace!("http should log| {}| {}", id, result);
                        let tosend = if result {
                            self.get_http_log_path(id).await
                        } else {
                            None
                        };

                        let response = CommanderResponse::HttpLog(tosend);
                        (id, response)
                    }

                    /* Associated Values:
                     *      ct  : ContentType
                     *
                     * Steps:
                     *      1. if config is_some call
                     *         config.in_excluded_content_types() with
                     *         ContentType
                     *
                     *      2. if true, call self.get_http_log_path() with id
                     *         to get new path and Sender<CommanderToHistory>
                     */
                    CommanderRequest::ShouldLogHttpCt(id, ct) => {
                        let result =
                            self.config
                                .as_ref()
                                .is_none_or(|config| {
                                    config.in_excluded_content_types(ct)
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

    /* Description:
     *      Method to get the path for http log
     *
     * Steps:
     *      1. Clone history_path and append self.http_log_index
     *
     *      2. Create directory with new path,
     *          a. if success increment self.http_log_index and return
     *          Some((index, path, Sender<CommanderToHistory>))
     *          b. else return None
     */

    pub async fn get_http_log_path(
        &mut self,
        id: usize,
    ) -> Option<(usize, PathBuf, Sender<CommanderToHistory>)> {
        let mut path = self.comm_history.path();
        path.push(
            self.comm_history
                .http_log_index
                .to_string(),
        ); //2
        trace!("log path| {}| {}", id, path.to_string_lossy());
        match create_dir(&path).await {
            Ok(_) => {
                let result = Some((
                    self.comm_history.http_log_index,
                    path,
                    self.comm_history.to_history.clone(),
                ));
                self.comm_history.http_log_index += 1;
                result
            }
            // Error printed, so we can send None to soldier
            // Should check Folder with index already exists ?
            Err(e) => {
                error!("creating directory| {:?}", e);
                None
            }
        }
    }

    /* Error:
     *      CommunicateError::NoId
     *      CommunicateError::Send
     */

    pub async fn empty_resume_queue(
        &mut self,
    ) -> Result<(), CommunicateError> {
        // http
        self.soldiers
            .broadcast_none_http(
                self.comm_interceptor
                    .http_queue_as_mut(),
            )
            .await?;
        trace!("http queue emptied");

        // wreq
        self.soldiers
            .broadcast_none_wreq(
                self.comm_interceptor
                    .wreq_queue_as_mut(),
            )
            .await?;
        trace!("wreq queue emptied");

        // wres
        self.soldiers
            .broadcast_none_wres(
                self.comm_interceptor
                    .wres_queue_as_mut(),
            )
            .await?;
        trace!("wres queue emptied");
        Ok(())
    }

    /* Steps:
     *      Match request
     *      1. If toggle, call interceptor_handle.toggle().
     *
     *      2. If intercept status is false, empty the interceptor queue
     *      (msgs which have been intercepted).
     *
     *      3. If Forward, call interceptor_handle.forward()
     *
     *      4. if resume,
     *          a. if wreq, and need_response, set_ws_need_response(id)
     *
     *      5. For resume and drop msgs, get log_id, file_type, response
     *
     *      6. Get connection id from log_id, by calling conn_id_from_log_id
     *
     *      7. Send response to the respective soldier
     */

    pub async fn handle_interceptor(
        &mut self,
        msg: InterUIOps,
    ) -> Result<(), CommunicateError> {
        match msg {
            InterUIOps::Toggle => {
                trace!("toggle");
                self.comm_interceptor.toggle();
                // 2. If toggle is false empty the queue
                if !self.comm_interceptor.status() {
                    self.empty_resume_queue().await?;
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
                let conn_id = self
                    .comm_interceptor
                    .conn_id_from_log_id(log_id, &ft)?;

                if response.wreq_need_response() {
                    self.soldiers
                        .set_ws_need_response(conn_id)?;
                }
                self.soldiers
                    .send_response_ft(conn_id, &ft, response)
                    .await
            }
        }
    }

    pub async fn handle_history(
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

/* Description:
 *      Runs the Commander .
 *      Continuously looks for messages from
 *          soldiers
 *          interceptor
 *          repeater
 *          history
 *
 * Steps:
 *      ----- loop -----
 *          ----- select -----
 *              Soldier      => handle_soldier
 *              Interceptor  => handle_interceptor
 *              History      => handle_history
 *              Repeater     => forward
 *              Token        => return
 */

pub async fn run_commander(mut comm: Commander, token: CancellationToken) {
    debug!("[+] commander started");
    loop {
        tokio::select! {
            soldier_msg = comm.comm_soldiers.recv() => {
                if let Some(smsg) = soldier_msg
                    && let Err(e) = comm.handle_soldier(smsg).await {
                        error!("soldier| {:?}", e);
                    }
            }
            interceptor_msg = comm.comm_interceptor.from_interceptor.recv() => {
                if let Some(imsg) = interceptor_msg
                    && let Err(e) = comm.handle_interceptor(imsg).await {
                        error!("interceptor| {:?}", e);
                    }
            }
            history_msg = comm.comm_history.from_history.recv() => {
                if let Some(msg) = history_msg
                    && let Err(e) = comm.handle_history(msg).await {
                        error!("history| {:?}", e);
                    }
            }
            repeater_msg = comm.comm_repeater.from_repeater.recv() => {
                if let Some(msg) = repeater_msg
                    && let Err(e) = comm.forward(msg).await {
                        error!("repeater| {:?}", e);
                    }
            }
            _ = token.cancelled() => {
                trace!("cancelled");
                return
            }
        }
    }
}
