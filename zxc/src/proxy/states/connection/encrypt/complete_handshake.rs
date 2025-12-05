use openssl::hash::MessageDigest;
use openssl::x509::X509;
use rustls_pki_types::UnixTime;
use tokio::sync::mpsc::Receiver;
use tokio_rustls::StartHandshake;
pub use tokio_rustls::client::TlsStream as ClientTlsStream;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::client::WebPkiServerVerifier;
use tokio_rustls::rustls::client::danger::ServerCertVerifier;
pub use tokio_rustls::server::TlsStream as ServerTlsStream;

use super::*;
use crate::commander::CommanderResponse;
use crate::commander::captain_crypto::error::CertError;
use crate::proxy::states::StateError;

/* Description:
 *      Completes client side handshake.
 *
 * Steps:
 *      1. Get server certificates.
 *
 *      2. Get ServerName.
 *
 *      3. Get WebPkiServerVerifier.
 *          a. Build CommanderRequest::GetVerifier
 *          b. send request to commander and recv response,
 *          CommanderResponse::Verifier
 *          c. Get Arc<WebPkiServerVerifier> from response
 *          [ TryFrom trait implemented in response/convert.rs ]
 *
 *      4. Verify server certificate.
 *
 *      5. Get MessageDigest::SHA256 from server certificate.
 *
 *      6. Check if Certificate already exists in store.
 *          a. Build Communicate::CheckCertificate request.
 *          b. send request to commander and recv response,
 *          CommanderResponse::ServerConfig
 *          c. Get Option<ServerConfig> from response
 *          [ TryFrom trait implemented in response/convert.rs ]
 *          c. If config recvd complete handshake
 *
 *      7. If no config, Generate New Config.
 *          a. Convert cert_chain into owned Vec<CertificateDer<'static>>
 *          b. Build Communicate::GenNewCert request.
 *          c. send request to commander and recv response,
 *          CommanderResponse::NewCertificate
 *          d. Get Result<Arc<ServerConfig>, CertError> from response
 *          [ TryFrom trait implemented in response/convert.rs]
 *
 *      8. Complete the handshake by calling into_stream() with
 *         server_config as arg on client stream.
 *
 * Error:
 *      StateError::NoPeerCertificate   [1]
 *      StateError::CommanderSend       [3] [6] [7]
 *      StateError::CommanderRecv       [3] [6] [7]
 *      StateError::Serial              [5]
 *      StateError::ClientEncrypt       [8]
 */

const COMPLETE_HANDSHAKE: &str = "Complete Handshake";

impl<T> Connection<StartHandshake<T>, ClientTlsStream<Tcp>>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin, // Stream
{
    pub async fn complete_handshake(
        self,
        recvr: &mut Receiver<CommanderResponse>,
        server_info: &ServerInfo,
    ) -> Result<Connection<ServerTlsStream<T>, ClientTlsStream<Tcp>>, StateError>
    {
        // 1. Get server certificates
        let cert_chain = self
            .writer
            .get_ref()
            .1
            .peer_certificates()
            .ok_or(StateError::NoPeerCertificate)?;

        // 2. Get ServerName
        let server_name = server_info.sni();

        // 3. Get WebPkiServerVerifier
        let req = CommanderRequest::GetVerifier(self.id);
        self.commander.send(req).await?;
        let res = recvr
            .recv()
            .await
            .ok_or(StateError::CommanderRecv(COMPLETE_HANDSHAKE))?;

        let verifier = Arc::<WebPkiServerVerifier>::try_from(res)?;

        // 4. Verify
        let verify = verifier
            .verify_server_cert(
                &cert_chain[0],
                &cert_chain[1..],
                server_name,
                &[],
                UnixTime::now(),
            )
            .is_ok();

        // 5. Get Hash
        let digest = X509::from_der(cert_chain[0].as_ref())?
            .digest(MessageDigest::sha256())?;

        // 6. Check if cert already exists
        let req = CommanderRequest::CheckCertificate(self.id, verify, digest);
        self.commander.send(req).await?;
        let res = recvr
            .recv()
            .await
            .ok_or(StateError::CommanderRecv(COMPLETE_HANDSHAKE))?;
        let recvd_config = Option::<Arc<ServerConfig>>::try_from(res)?;

        let server_config = match recvd_config {
            Some(config) => config,
            // 7. If no config
            _ => {
                let owned_cert_chain = cert_chain
                    .iter()
                    .cloned()
                    .map(|x| x.into_owned())
                    .collect();
                let req = CommanderRequest::GenNewCert(
                    self.id,
                    verify,
                    digest,
                    owned_cert_chain,
                );
                self.commander.send(req).await?;

                let res = recvr
                    .recv()
                    .await
                    .ok_or(StateError::CommanderRecv(COMPLETE_HANDSHAKE))?;
                let result =
                    Result::<Arc<ServerConfig>, CertError>::try_from(res)?;
                result?
            }
        };

        // 8. Complete Handshake
        let stream = self
            .reader
            .into_stream(server_config)
            .await
            .map_err(StateError::ClientEncrypt)?;
        Ok(Connection {
            id: self.id,
            commander: self.commander,
            reader: stream,
            writer: self.writer,
            buf: self.buf,
            frame: self.frame,
        })
    }
}
