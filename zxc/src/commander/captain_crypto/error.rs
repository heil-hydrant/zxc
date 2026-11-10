use std::{env, io};

use thiserror::Error;
use tokio_rustls::rustls;
use tokio_rustls::rustls::server::VerifierBuilderError;

#[derive(Debug, Error)]
pub enum CertError {
    #[error("certificate generation: {0}")]
    Rcgen(#[from] rcgen::Error),
    #[error("TLS configuration: {0}")]
    Rustls(#[from] rustls::Error),
    #[error("x509 parse: {0}")]
    X509(String),
}

#[derive(Debug, Error)]
pub enum CryptoBuildError {
    #[error("home directory lookup: {0}")]
    Var(#[from] env::VarError),
    #[error("verifier build: {0}")]
    VerifierBuild(#[from] VerifierBuilderError),
    #[error("file read: {0}")]
    Read(#[from] io::Error),
    #[error("certificate generation: {0}")]
    Rcgen(#[from] rcgen::Error),
    #[error("private key read: {0}")]
    ReadPrivateKey(String),
    #[error("private.key not found")]
    NoKeyFound,
    #[error("unknown private key type")]
    UnknownPrivateKeyType,
}
