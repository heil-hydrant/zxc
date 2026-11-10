use std::num::ParseIntError;
use std::str::Utf8Error;

use header_plz::uri::InvalidUri;
use rustls_pki_types::InvalidDnsNameError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AddressError {
    #[error("address is not valid UTF-8: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("address has no host: {0}")]
    NoHost(String),
    #[error("invalid port: {0}")]
    PortParse(#[from] ParseIntError),
    #[error("invalid DNS name: {0}")]
    InvalidDnsName(#[from] InvalidDnsNameError),
    #[error("invalid URI: {0}")]
    InvalidUri(#[from] InvalidUri),
    #[error("address has an empty host")]
    EmptyHost,
}
