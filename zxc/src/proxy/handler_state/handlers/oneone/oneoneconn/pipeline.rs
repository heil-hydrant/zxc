use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

use bytes::BytesMut;
use decompression_plz::MultiDecompressErrorReason;
use header_plz::body_headers::parse::ParseBodyHeaders;
use header_plz::uri::path::PathAndQuery;
use header_plz::{
    MessageHead, OneHeader, OneInfoLine, OneRequestLine, OneResponseLine
};
use http_plz::{OneMessageHead, OneOne};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tracing::{error, trace};

use super::OneOneConn;
use crate::commander::*;
use crate::history::*;
use crate::interceptor::*;
use crate::io::socket::establish_connection;
use crate::io::write_and_flush;
use crate::proxy::handler_state::handlers::read_http;
use crate::proxy::prelude::*;
use crate::proxy::states::ClientTlsStream;

// --- AddRaw ---
impl<T, E, U> AddRaw for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    fn add_raw(&mut self, buf: BytesMut) {
        self.msg_bytes = Some(buf);
    }
}

// --- BytesToMsg ---
impl<T, E, U> BytesToMsg for OneOneConn<T, E, U>
where
    U: OneInfoLine + Debug,
    OneMessageHead<U>: ParseBodyHeaders,
{
    type Msg = OneOne<U>;

    fn parse_bytes(&self, buf: BytesMut) -> Result<Self::Msg, UpdateMsgError> {
        Ok(OneOne::<U>::try_from(buf)?)
    }

    fn add_msg(&mut self, msg: Self::Msg) {
        self.msg = Some(msg);
    }
}

// --- DropMsg ---
// Http/1.1 always close connection
impl<T, E, U> DropMsg for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    #[inline(always)]
    fn continue_on_drop() -> Result<(), ProxyStateError> {
        Err(ProxyStateError::Drop)
    }
}

// --- MsgToBytes ---
impl<T, E, U> MsgToBytes for OneOneConn<T, E, U>
where
    U: OneInfoLine + Debug,
    OneMessageHead<U>: ParseBodyHeaders,
{
    fn msg_to_bytes(&mut self) {
        // safe to unwrap
        self.msg_bytes = Some(self.msg.take().unwrap().into_bytes());
    }
}

// --- Intercept ---
impl<T, E> Intercept for OneOneConn<T, E, OneRequestLine> {
    fn get_inter_info(&self) -> InterToUI {
        let info = ServerInfoJson::from(&self.server_info);
        InterToUI::build_http_req(self.log_id, Some(info))
    }
}

impl<T, E> Intercept for OneOneConn<T, E, OneResponseLine> {
    fn get_inter_info(&self) -> InterToUI {
        InterToUI::build_http_res(self.log_id)
    }
}

// --- UpdateLogExt ---
impl<T, E> UpdateLogExt for OneOneConn<T, E, OneRequestLine> {
    fn update_extension(&mut self) {
        update_http_req_ext(&mut self.path, self.log_id);
    }
}

impl<T, E> UpdateLogExt for OneOneConn<T, E, OneResponseLine> {
    fn update_extension(&mut self) {
        update_http_res_ext(&mut self.path);
    }
}

// --- ReadWrite ---
#[derive(Debug, Error)]
pub enum OneOneRWError {
    #[error("read| {0}")]
    Read(io::Error),
    #[error("write| {0}")]
    Write(io::Error),
    #[error("request parse| {0}")]
    RequestParse(#[from] oneone_plz::error::Error<OneRequestLine>),
    #[error("response parse| {0}")]
    ResponseParse(#[from] oneone_plz::error::Error<OneResponseLine>),
    #[error("decompress| {0}")]
    Decompress(#[from] MultiDecompressErrorReason),
    #[error("incorrect state| {0}")]
    IncorrectState(#[from] oneone_plz::error::IncorrectState),
}

impl<T, E, U> ReadWrite for OneOneConn<T, E, U>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
    U: OneInfoLine + Debug,
    MessageHead<U, OneHeader>: ParseBodyHeaders,
    OneOneRWError: From<oneone_plz::error::Error<U>>,
{
    type Error = OneOneRWError;
    type State = ProxyState<Self>;

    async fn read(mut self) -> Result<ProxyState<Self>, OneOneRWError> {
        trace!("reading");
        match read_http::<T, U>(&mut self.reader, &mut self.buf).await {
            Ok(frame) => {
                self.msg = Some(frame);
                Ok(ProxyState::ShouldLog(self))
            }
            Err(e) => match self.role() {
                Role::Client => Ok(ProxyState::ServerClose(self, e.into())),
                Role::Server => Err(e),
            },
        }
    }

    async fn write(mut self) -> Result<ProxyState<Self>, OneOneRWError> {
        trace!("writing");
        if let Err(e) =
            write_and_flush(&mut self.writer, self.msg_bytes.as_ref().unwrap())
                .await
        {
            let e = OneOneRWError::Write(e);
            match self.role() {
                Role::Server => {
                    return Ok(ProxyState::ServerClose(self, e.into()));
                }
                Role::Client => return Err(e),
            }
        }
        Ok(ProxyState::End(self))
    }
}

// --- Reconnect ---
impl<T> Reconnect for OneOneConn<T, TcpStream, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn can_reconnect(&self, server_info: &ServerInfo) -> bool {
        !server_info.is_tls()
    }

    async fn reconnect(&mut self) -> Result<(), ProxyStateError> {
        self.writer = establish_connection(self.address()).await?;
        Ok(())
    }
}

impl<T> Reconnect for OneOneConn<T, ClientTlsStream<TcpStream>, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn can_reconnect(&self, server_info: &ServerInfo) -> bool {
        server_info.is_tls()
    }

    async fn reconnect(&mut self) -> Result<(), ProxyStateError> {
        let tcp = establish_connection(self.address()).await?;
        trace!("Reconnected");
        let server_name = self.server_info.sni().clone();
        let tls = server_encrypt(
            self.id.conn_id(),
            &mut self.commander_sendr,
            &mut self.commander_recvr,
            server_name,
            tcp,
            false, // TODO
        )
        .await?;
        trace!("Encrypted");
        self.writer = tls;
        Ok(())
    }
}

// --- Rewrite ---
impl<T, E, U> Rewrite for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    fn file_and_data(&mut self) -> (&mut File, &[u8]) {
        (self.file.as_mut().unwrap(), self.msg_bytes.as_ref().unwrap()) // safe to unwrap
    }
}

// --- ShouldIntercept ---
impl<T, E> ShouldIntercept for OneOneConn<T, E, OneRequestLine> {}

impl<T, E> ShouldIntercept for OneOneConn<T, E, OneResponseLine> {
    #[inline(always)]
    fn should_intercept(&self) -> Option<bool> {
        Some(self.need_response)
    }
}

// --- CanLog & ShouldLog & HttpRequestLog ---
impl<T, E, U> CanLog for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    #[inline(always)]
    fn can_log(&self) -> bool {
        self.path.is_some()
    }
}

impl<T, E> HttpRequestLog for OneOneConn<T, E, OneRequestLine> {
    fn id(&self) -> SoldierId {
        self.id
    }

    fn log_id_mut(&mut self) -> &mut usize {
        &mut self.log_id
    }

    fn path_mut(&mut self) -> &mut Option<PathBuf> {
        &mut self.path
    }

    fn history_sendr_mut(
        &mut self,
    ) -> &mut Option<Sender<CommanderToHistory>> {
        &mut self.history_sendr
    }

    fn http_commander_log_request(&self) -> Option<CommanderRequest> {
        let frame = self.msg.as_ref().unwrap();
        http_log_request(
            self.id,
            &frame.method_enum(),
            frame.header_map(),
            || {
                let uri = frame.uri_as_string();
                trace!("URI| {}", uri);
                match PathAndQuery::from_str(&uri) {
                    Ok(uri) => uri.path().to_string(),
                    Err(e) => {
                        error!("uri parse: {}", e);
                        String::new()
                    }
                }
            },
        )
    }

    #[inline(always)]
    fn needs_msg_to_bytes(&self) -> bool {
        true
    }
}

impl<T, E> ShouldLog for OneOneConn<T, E, OneResponseLine> {
    type LogResult = ();

    #[inline(always)]
    fn needs_msg_to_bytes(&self) -> bool {
        true
    }
}

// --- ShouldRewrite ---
impl<T, E, U> ShouldRewrite for OneOneConn<T, E, U>
where
    U: OneInfoLine,
{
    #[inline(always)]
    fn should_rewrite(&self, resume_info: &ResumeInfo) -> bool {
        resume_info.update()
    }
}

// --- UpdateResumeInfo ---
impl<T, E> UpdateResumeInfo for OneOneConn<T, E, OneRequestLine> {
    fn update_resume_info(&mut self, resume_info: &ResumeInfo) {
        self.need_response = resume_info.need_response();
    }
}

// Blanket implementation
impl<T, E> UpdateResumeInfo for OneOneConn<T, E, OneResponseLine> {
    #[inline(always)]
    fn update_resume_info(&mut self, _: &ResumeInfo) {}
}

// --- GetHistory ---
impl<T, E> GetHistory for OneOneConn<T, E, OneRequestLine> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let req = self.msg.as_ref().unwrap(); // safe to unwrap
        let method = std::str::from_utf8(req.method_bytes()).unwrap_or("GET");
        HistoryEnum::http_request(
            self.log_id,
            &self.server_info,
            method,
            req.uri_as_string(),
        )
    }
}

impl<T, E> GetHistory for OneOneConn<T, E, OneResponseLine> {
    fn get_history(&self) -> HistoryEnum<'_> {
        let res = self.msg.as_ref().unwrap(); // safe to unwrap
        let status = res
            .status()
            .map(|s| s.as_u16())
            .unwrap_or_default();
        HistoryEnum::http_response(self.log_id, status, res.content_length())
    }
}
