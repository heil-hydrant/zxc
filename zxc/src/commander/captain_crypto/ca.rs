use std::fs::read_to_string;
use std::sync::Arc;

use openssl::hash::DigestBytes;
use rcgen::{CertificateParams, Issuer, KeyPair};
use tokio_rustls::rustls::ServerConfig;

use super::CryptoBuildError;
use crate::config::global::parser::global_config_path;

/* Destiption:
 *      CA Certificate is used to generate self signeed tls certificates for
 * the domains
 *
 * There are two CA's:
 *      1. Trusted      : user generated and trusted
 *      2. Untrusted    : per session generated
 *
 * TODO:
 *      Use HashMap instead of Vec
 *      https://github.com/sfackler/rust-openssl/pull/2299
 */

pub struct CA {
    issuer: Issuer<'static, KeyPair>,
    store: Vec<(DigestBytes, Arc<ServerConfig>)>,
}

impl CA {
    // User generated private key and certificate
    // Read from file $HOME/.config/zxc/zxca.crt
    pub fn trusted(key_pair: KeyPair) -> Result<CA, CryptoBuildError> {
        let mut cert_path = global_config_path()?;
        cert_path.push("zxca.crt");
        let cert_str = read_to_string(cert_path)?;
        let issuer = Issuer::from_ca_cert_pem(&cert_str, key_pair)?;
        Ok(CA {
            issuer,
            store: Vec::new(),
        })
    }

    // Per session CA Certificate
    pub fn untrusted(key_pair: KeyPair) -> Result<CA, CryptoBuildError> {
        let issuer = Issuer::new(CertificateParams::default(), key_pair);
        Ok(CA {
            issuer,
            store: Vec::new(),
        })
    }

    pub fn signer(&self) -> &Issuer<'_, KeyPair> {
        &self.issuer
    }

    pub fn store(&self) -> &Vec<(DigestBytes, Arc<ServerConfig>)> {
        &self.store
    }

    pub fn add_config(
        &mut self,
        digest: DigestBytes,
        config: Arc<ServerConfig>,
    ) {
        self.store.push((digest, config));
    }
}
