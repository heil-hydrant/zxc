use thiserror::Error;

use crate::io;

#[derive(Debug, Error)]
pub enum AddonError {
    #[error(transparent)]
    Ui(#[from] crate::run::boundary::UiError),
    #[error("unexpected forward info: {0:?}")]
    WrongMsg(crate::core::ForwardInfo),
    #[error("addon not found: {0}")]
    AddonNotFound(String),
    #[error("h2 request: {0}")]
    H2Request(String),
    #[error(transparent)]
    RequestFile(#[from] io::FileErrorInfo),
    #[error(transparent)]
    DirError(#[from] io::DirError),
}

crate::impl_ui_error!(AddonError);
