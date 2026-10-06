use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::Arc;

use rcgen::{CertificateParams, Issuer, KeyPair};
use tokio_rustls::rustls::ServerConfig;

use super::CryptoBuildError;
use crate::commander::captain_crypto::{CertDigest, NegotiatedAlpn};
use crate::config::global::parser::global_config_path;

// There are two CA's:
//      1. Trusted      : user generated and trusted
//      2. Untrusted    : generated per session

pub struct CA {
    issuer: Issuer<'static, KeyPair>,
    store: HashMap<(CertDigest, NegotiatedAlpn), Arc<ServerConfig>>,
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
            store: HashMap::new(),
        })
    }

    // Per session CA Certificate
    pub fn untrusted(key_pair: KeyPair) -> Result<CA, CryptoBuildError> {
        let issuer = Issuer::new(CertificateParams::default(), key_pair);
        Ok(CA {
            issuer,
            store: HashMap::new(),
        })
    }

    pub fn signer(&self) -> &Issuer<'_, KeyPair> {
        &self.issuer
    }

    pub fn store(
        &self,
    ) -> &HashMap<(CertDigest, NegotiatedAlpn), Arc<ServerConfig>> {
        &self.store
    }

    pub fn add_config(
        &mut self,
        digest: CertDigest,
        alpn: NegotiatedAlpn,
        config: Arc<ServerConfig>,
    ) {
        self.store
            .insert((digest, alpn), config);
    }
}
