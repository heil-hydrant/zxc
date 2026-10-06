use std::sync::Arc;

use rcgen::{CertificateParams, Issuer, KeyPair, SanType};
use rustls_pki_types::PrivateKeyDer;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::client::WebPkiServerVerifier;
use tokio_rustls::rustls::client::danger::ServerCertVerifier;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tokio_rustls::rustls::{
    ClientConfig, RootCertStore, ServerConfig, {self},
};
use tracing::trace;
use x509_parser::asn1_rs::FromDer;

mod ca;
pub mod error;
mod private_key;
mod verifier;

use ca::*;
use error::*;
use private_key::{read_private, str_to_private};
use verifier::*;

pub type CertDigest = [u8; 32];

pub const ALPN_H1: &[u8] = b"http/1.1";
pub const ALPN_H2: &[u8] = b"h2";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NegotiatedAlpn {
    None,
    Http1,
    H2,
}

impl NegotiatedAlpn {
    pub fn from_bytes(alpn: Option<&[u8]>) -> Self {
        match alpn {
            None => NegotiatedAlpn::None,
            Some(ALPN_H2) => NegotiatedAlpn::H2,
            Some(_) => NegotiatedAlpn::Http1,
        }
    }

    pub fn as_bytes(self) -> Option<&'static [u8]> {
        match self {
            NegotiatedAlpn::None => None,
            NegotiatedAlpn::Http1 => Some(ALPN_H1),
            NegotiatedAlpn::H2 => Some(ALPN_H2),
        }
    }
}

pub struct CaptainCrypto {
    connector: Arc<TlsConnector>,
    key_pair: KeyPair,
    private_key: PrivateKeyDer<'static>,
    trusted_ca: CA,
    untrusted_ca: CA,
    web_pki: Arc<WebPkiServerVerifier>,
}

impl CaptainCrypto {
    pub fn new() -> Result<Self, CryptoBuildError> {
        let root_cert_store = RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned(),
        );
        let web_pki =
            WebPkiServerVerifier::builder(root_cert_store.into()).build()?;
        let verifier = CertVerifier::new(web_pki.supported_verify_schemes());
        let mut client_config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth();
        client_config.alpn_protocols =
            vec![ALPN_H2.to_vec(), ALPN_H1.to_vec()];
        let tls_connector = TlsConnector::from(Arc::new(client_config));
        let connector = Arc::new(tls_connector);

        let pk_str = read_private()?;
        let key_pair = KeyPair::from_pem(&pk_str)?;
        let trusted_ca = CA::trusted(KeyPair::from_pem(&pk_str)?)?;
        let untrusted_ca = CA::untrusted(KeyPair::from_pem(&pk_str)?)?;
        let private_key = str_to_private(&pk_str)?;
        Ok(CaptainCrypto {
            connector,
            key_pair,
            private_key,
            trusted_ca,
            untrusted_ca,
            web_pki,
        })
    }

    pub fn get_connector(&self) -> Arc<TlsConnector> {
        self.connector.clone()
    }

    pub fn get_verifier(&self) -> Arc<WebPkiServerVerifier> {
        self.web_pki.clone()
    }

    pub fn check_server_config_cache(
        &self,
        verified: bool,
        digest: CertDigest,
        alpn: NegotiatedAlpn,
    ) -> Option<Arc<ServerConfig>> {
        let cert_store = if verified {
            trace!("trusted");
            &self.trusted_ca.store()
        } else {
            trace!("untrusted");
            &self.untrusted_ca.store()
        };
        cert_store.get(&(digest, alpn)).cloned()
    }

    pub fn get_server_config(
        &mut self,
        verified: bool,
        digest: CertDigest,
        cert: Vec<CertificateDer<'static>>,
        alpn: NegotiatedAlpn,
    ) -> Result<Arc<ServerConfig>, CertError> {
        let ca = if verified {
            trace!("new cert| ca| Y");
            &mut self.trusted_ca
        } else {
            trace!("new cert| ca| N");
            &mut self.untrusted_ca
        };
        let cert = generate_domain_cert(&self.key_pair, cert, ca.signer())?;

        let config =
            generate_server_config(cert, self.private_key.clone_key(), alpn)?;
        trace!("server config| Y");

        let arc_config = Arc::new(config);
        ca.add_config(digest, alpn, arc_config.clone());
        Ok(arc_config)
    }
}

fn generate_domain_cert(
    keypair: &KeyPair,
    cert: Vec<CertificateDer<'static>>,
    signer: &Issuer<'_, KeyPair>,
) -> Result<CertificateDer<'static>, CertError> {
    use std::net::IpAddr;

    use rcgen::string::Ia5String;
    use x509_parser::extensions::GeneralName;

    let (_, real_cert) =
        x509_parser::certificate::X509Certificate::from_der(cert[0].as_ref())
            .map_err(|e| CertError::X509(e.to_string()))?;
    let subject_alt_names = real_cert
        .subject_alternative_name()
        .map_err(|e| CertError::X509(e.to_string()))?
        .map(|ext| {
            ext.value
                .general_names
                .iter()
                .filter_map(|name| match name {
                    GeneralName::DNSName(dns) => {
                        Some(Ia5String::try_from(*dns).map(SanType::DnsName))
                    }
                    GeneralName::IPAddress(bytes) => match bytes.len() {
                        4 => <[u8; 4]>::try_from(*bytes)
                            .ok()
                            .map(|arr| {
                                Ok(SanType::IpAddress(IpAddr::from(arr)))
                            }),
                        16 => <[u8; 16]>::try_from(*bytes)
                            .ok()
                            .map(|arr| {
                                Ok(SanType::IpAddress(IpAddr::from(arr)))
                            }),
                        _ => None,
                    },
                    _ => None,
                })
                .collect::<Result<Vec<_>, rcgen::Error>>()
        })
        .transpose()?;

    let mut cert_params = CertificateParams::default();
    if let Some(alt_names) = subject_alt_names {
        cert_params.subject_alt_names = alt_names;
    }
    let certificate: CertificateDer<'static> = cert_params
        .signed_by(keypair, signer)?
        .into();
    Ok(certificate)
}

fn generate_server_config(
    cert: CertificateDer<'static>,
    private_key: PrivateKeyDer<'static>,
    alpn: NegotiatedAlpn,
) -> Result<ServerConfig, rustls::Error> {
    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], private_key)?;
    server_config.alpn_protocols = alpn
        .as_bytes()
        .map(|p| vec![p.to_vec()])
        .unwrap_or_default();
    Ok(server_config)
}
