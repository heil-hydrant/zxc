use http_plz::MsgParseErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyUpdateFrameError {
    #[error("updating frame")]
    HttpFrame(#[from] MsgParseErr),
    #[error("invalid ws frame")]
    InvalidWsFrame,
}
