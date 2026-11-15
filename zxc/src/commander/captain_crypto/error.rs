use std::path::PathBuf;
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
    #[error("file read: {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("certificate generation: {0}")]
    Rcgen(#[from] rcgen::Error),
    #[error("private key parse: {0}")]
    ReadPrivateKey(String),
    #[error("private key: no PKCS#8 key found")]
    NoKeyFound,
    #[error("unknown private key type")]
    UnknownPrivateKeyType,
}
