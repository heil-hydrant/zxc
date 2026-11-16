use std::collections::{HashMap, VecDeque};
use std::fs::read_to_string;
use std::sync::Arc;

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa,
    Issuer, KeyPair, KeyUsagePurpose,
};
use tokio_rustls::rustls::ServerConfig;

use super::{Alpn, CertDigest, CryptoBuildError};
use crate::config::global_config_path;

// There are two CA's:
//      1. Trusted      : user generated and trusted
//      2. Untrusted    : generated per session

// Upper bound on cached server configs per CA. Each entry retains a
// generated leaf cert chain and a cloned private key, so this also
// bounds retained key material.
#[cfg(not(test))]
const MAX_CACHED_CONFIGS: usize = 1024;

// Small under test so eviction is cheap to exercise.
#[cfg(test)]
const MAX_CACHED_CONFIGS: usize = 8;

pub struct CA {
    issuer: Issuer<'static, KeyPair>,
    store: HashMap<(CertDigest, Alpn), Arc<ServerConfig>>,
    order: VecDeque<(CertDigest, Alpn)>,
}

impl CA {
    // User generated private key and certificate
    // Read from file $HOME/.config/zxc/zxca.crt
    pub fn trusted(key_pair: KeyPair) -> Result<CA, CryptoBuildError> {
        let mut cert_path = global_config_path()?;
        cert_path.push("zxca.crt");
        let cert_str = read_to_string(&cert_path).map_err(|source| {
            CryptoBuildError::Read {
                path: cert_path,
                source,
            }
        })?;
        let issuer = Issuer::from_ca_cert_pem(&cert_str, key_pair)?;
        Ok(CA {
            issuer,
            store: HashMap::new(),
            order: VecDeque::new(),
        })
    }

    // Per session CA Certificate
    pub fn untrusted(key_pair: KeyPair) -> Result<CA, CryptoBuildError> {
        let issuer = Issuer::new(CertificateParams::default(), key_pair);
        Ok(CA {
            issuer,
            store: HashMap::new(),
            order: VecDeque::new(),
        })
    }

    // Temporary CA generated when the user CA is unavailable
    pub fn ephemeral() -> Result<CA, CryptoBuildError> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, "zxc temporary session CA");
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;
        let issuer = Issuer::from_ca_cert_der(cert.der(), key_pair)?;
        Ok(CA {
            issuer,
            store: HashMap::new(),
            order: VecDeque::new(),
        })
    }

    pub fn signer(&self) -> &Issuer<'_, KeyPair> {
        &self.issuer
    }

    pub fn store(&self) -> &HashMap<(CertDigest, Alpn), Arc<ServerConfig>> {
        &self.store
    }

    pub fn add_config(
        &mut self,
        digest: CertDigest,
        alpn: Alpn,
        config: Arc<ServerConfig>,
    ) {
        let key = (digest, alpn);
        // Only new keys enter the eviction queue. Replacing an existing
        // entry must not push a duplicate, or a later eviction would
        // drop a key that is still the newest.
        if self.store.insert(key, config).is_none() {
            self.order.push_back(key);
        }
        // FIFO: `order` stays a permutation of `store`'s keys.
        while self.store.len() > MAX_CACHED_CONFIGS {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.store.remove(&oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use rcgen::{CertificateParams, KeyPair};
    use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

    use super::*;
    use crate::commander::captain_crypto::generate_server_config;

    fn dummy_config() -> Arc<ServerConfig> {
        let key = KeyPair::generate().unwrap();
        let cert = CertificateParams::default()
            .self_signed(&key)
            .unwrap()
            .der()
            .clone();
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            key.serialize_der(),
        ));
        let config =
            generate_server_config(cert, private_key, Alpn::Http1).unwrap();
        Arc::new(config)
    }

    #[test]
    fn test_cache_evicts_oldest() {
        let mut ca = CA::ephemeral().unwrap();
        let cap = MAX_CACHED_CONFIGS;
        for i in 0..cap + 2 {
            ca.add_config([i as u8; 32], Alpn::Http1, dummy_config());
        }

        assert_eq!(ca.store().len(), cap);
        assert_eq!(ca.order.len(), cap);
        let evicted_first = ([0u8; 32], Alpn::Http1);
        let evicted_second = ([1u8; 32], Alpn::Http1);
        let newest = ([(cap + 1) as u8; 32], Alpn::Http1);
        assert!(!ca.store().contains_key(&evicted_first));
        assert!(!ca.store().contains_key(&evicted_second));
        assert!(ca.store().contains_key(&newest));
    }

    #[test]
    fn test_cache_replace_keeps_bound() {
        let mut ca = CA::ephemeral().unwrap();
        let cap = MAX_CACHED_CONFIGS;
        for i in 0..cap {
            ca.add_config([i as u8; 32], Alpn::Http1, dummy_config());
        }

        let replaced = dummy_config();
        ca.add_config([0u8; 32], Alpn::Http1, replaced.clone());

        assert_eq!(ca.store().len(), cap);
        assert_eq!(ca.order.len(), cap);
        let key = ([0u8; 32], Alpn::Http1);
        let last = ([(cap - 1) as u8; 32], Alpn::Http1);
        assert!(ca.store().contains_key(&key));
        assert!(ca.store().contains_key(&last));
        assert!(Arc::ptr_eq(&replaced, ca.store().get(&key).unwrap()));
    }
}
