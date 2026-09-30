use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::{DigitallySignedStruct, Error, SignatureScheme};

/* Descrption:
 *      Custom Certificate verifier implements ServerCertVerifier trait.
 *      Does not verify the certificate.
 *      Always returns true.
 */

#[derive(Debug)]
pub struct CertVerifier {
    schemes: Vec<SignatureScheme>,
}

impl CertVerifier {
    pub fn new(schemes: Vec<SignatureScheme>) -> CertVerifier {
        CertVerifier {
            schemes,
        }
    }
}

// ServerCertVerifier trait blanket implementation
// Always returns true
impl ServerCertVerifier for CertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes.clone()
    }
}
