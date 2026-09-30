mod ca;
pub mod error;
mod verifier;
use std::sync::Arc;

use ca::*;
use error::*;
use openssl::hash::DigestBytes;
use openssl::x509::X509;
use rcgen::{CertificateParams, Issuer, KeyPair};
use rustls_pki_types::PrivateKeyDer;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::client::WebPkiServerVerifier;
use tokio_rustls::rustls::client::danger::ServerCertVerifier;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tokio_rustls::rustls::{
    ClientConfig, RootCertStore, ServerConfig, {self},
};
use tracing::trace;
use verifier::*;

mod private_key;
use private_key::{read_private, str_to_private};
use webpki_roots::TLS_SERVER_ROOTS;

const ALPN_H1: &[u8] = b"http/1.1";

pub struct CaptainCrypto {
    connector: Arc<TlsConnector>,
    key_pair: KeyPair,
    private_key: PrivateKeyDer<'static>,
    trusted_ca: CA,
    untrusted_ca: CA,
    web_pki: Arc<WebPkiServerVerifier>,
}

impl CaptainCrypto {
    /* Steps:
     *      1. Build WebPkiServerVerifier
     *          a. Build RootCertStore from TLS_SERVER_ROOTS
     *          b. Build WebPkiServerVerifier from RootCertStore
     *
     *      2. Build TlsConnector
     *          a. Get Vec<SignatureScheme> from WebPkiServerVerifier
     *
     *          b. Build ClientConfig with custom certificate verifier and
     *          Vec<SignatureScheme>
     *
     *          c. Build TlsConnector from ClientConfig
     *
     *      3. Read PrivateKey String from file, $HOME/.config/zxc/private.key
     *         by calling read_private().
     *
     *      4. Build KeyPair from PrivateKey String.
     *
     *      5. Build trusted_ca and untrusted_ca by methods CA::trusted()
     *         and CA::untrusted() with the KeyPair as arg
     *
     *      6. Convert PrivateKey (&str) to PrivateKeyDer
     *
     * Returns:
     *      Ok(CaptainCrypto)
     *
     * Errors:
     *      CryptoBuildError::VerifierBuild         [1.b]
     *      CryptoBuildError::Var                   [3]
     *      CryptoBuildError::Read                  [3]
     *      CryptoBuildError::UnknownPrivateKeyType [3]
     *      CryptoBuildError::Rcgen                 [4] [5]
     */

    pub fn new() -> Result<Self, CryptoBuildError> {
        // 1
        let root_cert_store =
            RootCertStore::from_iter(TLS_SERVER_ROOTS.iter().cloned());
        let web_pki =
            WebPkiServerVerifier::builder(root_cert_store.into()).build()?;

        // 2
        let supported_verify_schemes = web_pki.supported_verify_schemes();
        let verifier = CertVerifier::new(supported_verify_schemes);
        let client_config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth();
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

    /* Description:
     *      Check if a certificate already exists in the selected store.
     *
     * Steps:
     *      Select the Cert Store and Search,
     *          verified == true =>  Trusted
     *          verified == false => Untrusted
     */

    pub fn check_serial(
        &self,
        verified: bool,
        digest_to_check: DigestBytes,
    ) -> Option<Arc<ServerConfig>> {
        let cert_store = if verified {
            trace!("trusted");
            &self.trusted_ca.store()
        } else {
            trace!("untrusted");
            &self.untrusted_ca.store()
        };
        cert_store
            .iter()
            .find_map(|(digest, config)| {
                if digest.as_ref() == digest_to_check.as_ref() {
                    Some(config.clone())
                } else {
                    None
                }
            })
    }

    /* Description:
     *      Generate new certificate based on verification result from web_pki.
     *
     * Steps:
     *      1. Select CA based on verification result
     *          verified == true =>  Trusted
     *          verified == false => Untrusted
     *      2. Generate new domain cert using server cert and CA cert.
     *      3. Generate Server Config using generated cert and private key
     *      4. Push to the selected store
     *
     * Returns:
     *      Result<Arc<ServerConfig>, CertError>
     *
     * Error:
     *      CertError::Rcgen    [2]
     *      CertError::Rustls   [4]
     */

    pub fn generate_new_cert(
        &mut self,
        verified: bool,
        digest: DigestBytes,
        cert: Vec<CertificateDer<'static>>,
    ) -> Result<Arc<ServerConfig>, CertError> {
        // 1. Select CA based on verification result
        let ca = if verified {
            trace!("new cert| ca| Y");
            &mut self.trusted_ca
        } else {
            trace!("new cert| ca| N");
            &mut self.untrusted_ca
        };
        let gen_cert =
            generate_domain_cert(&self.key_pair, cert, ca.signer())?;

        let config =
            generate_server_config(gen_cert, self.private_key.clone_key())?;
        trace!("server config| Y");

        let arc_config = Arc::new(config);
        let tosend = arc_config.clone();
        ca.add_config(digest, arc_config);

        Ok(tosend)
    }
}

fn generate_domain_cert(
    keypair: &KeyPair,
    cert: Vec<CertificateDer<'static>>,
    signer: &Issuer<'_, KeyPair>,
) -> Result<CertificateDer<'static>, CertError> {
    let real_cert = X509::from_der(cert[0].as_ref())?;
    let sans: Vec<String> = real_cert
        .subject_alt_names()
        .map(|names| {
            names
                .iter()
                .filter_map(|n| n.dnsname().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let cert_params = CertificateParams::new(sans)?;
    let certificate: CertificateDer<'static> = cert_params
        .signed_by(keypair, signer)?
        .into();
    Ok(certificate)
}

fn generate_server_config(
    cert: CertificateDer<'static>,
    private_key: PrivateKeyDer<'static>,
) -> Result<ServerConfig, rustls::Error> {
    let certs = vec![cert];
    let mut server_conf = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)?;
    server_conf.alpn_protocols = vec![ALPN_H1.to_vec()];
    Ok(server_conf)
}
