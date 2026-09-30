use std::io;

use decompression_plz::MultiDecompressErrorReason;
use header_plz::body_headers::parse::ParseBodyHeaders;
use header_plz::{
    MessageHead, OneHeader, OneInfoLine, OneRequestLine, OneResponseLine,
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::trace;

use super::OneOneStruct;
use crate::io::write::write_and_flush;
use crate::proxy::handler_state::ProxyState;
use crate::proxy::handler_state::handlers::read_http;
use crate::proxy::handler_state::read_write::ReadWrite;
use crate::proxy::handler_state::role::{GetRole, Role};

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

impl<T, E, U> ReadWrite for OneOneStruct<T, E, U>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
    U: OneInfoLine + std::fmt::Debug,
    MessageHead<U, OneHeader>: ParseBodyHeaders,
    OneOneRWError: From<oneone_plz::error::Error<U>>,
{
    type Error = OneOneRWError;
    type State = ProxyState<Self>;

    /* Steps:
     *      1. call read_http() with reader and buf as args.
     *
     *      2. If Ok(frame) is returned, set the self.frame to the frame.
     *
     *      3. If Err(e) is returned, check role
     *
     *          a. if role is client, then server has closed return
     *          ProxyState::ServerClose
     *
     *          b. if role is server, return Err(e)
     *
     * Transition:
     *      Read -> ShouldLog | ServerClose
     *
     * Error:
     *      OneOneRWError::Read
     */

    async fn read(mut self) -> Result<ProxyState<Self>, OneOneRWError> {
        trace!("reading");
        match read_http::<T, U>(&mut self.reader, &mut self.buf).await {
            Ok(frame) => {
                self.frame = Some(frame);
                Ok(ProxyState::ShouldLog(self))
            }
            Err(e) => match self.role() {
                Role::Client => Ok(ProxyState::ServerClose(self, e.into())),
                Role::Server => Err(e),
            },
        }
    }

    /* Steps:
     *      1. call write_and_flush() with writer and data as args
     *
     *      2. If Err(e) is returned, check role
     *
     *          a. if role is server, then server has closed return
     *          ProxyState::ServerClose
     *
     *          b. if role is client, return Err(e)
     *
     *      3. Else Transition to End
     *
     * Transition:
     *      WriteResponse -> End | ServerClose
     *
     * Error:
     *      OneOneRWError::Write
     */

    async fn write(mut self) -> Result<ProxyState<Self>, OneOneRWError> {
        trace!("writing");
        if let Err(e) =
            write_and_flush(&mut self.writer, self.payload.as_ref().unwrap())
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
