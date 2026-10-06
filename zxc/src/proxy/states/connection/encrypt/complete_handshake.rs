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
use crate::commander::captain_crypto::{ALPN_H1, CertDigest, NegotiatedAlpn};
use crate::proxy::states::StateError;

const COMPLETE_HANDSHAKE: &str = "Complete Handshake";

impl<T> Connection<StartHandshake<T>, ClientTlsStream<Tcp>>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    pub async fn complete_handshake(
        self,
        recvr: &mut Receiver<CommanderResponse>,
        server_info: &ServerInfo,
    ) -> Result<Connection<ServerTlsStream<T>, ClientTlsStream<Tcp>>, StateError>
    {
        let cert_chain = self
            .writer
            .get_ref()
            .1
            .peer_certificates()
            .ok_or(StateError::NoPeerCertificate)?;

        let server_name = server_info.sni();

        let req = CommanderRequest::GetVerifier(self.id);
        self.commander.send(req).await?;
        let res = recvr
            .recv()
            .await
            .ok_or(StateError::CommanderRecv(COMPLETE_HANDSHAKE))?;
        let verifier = Arc::<WebPkiServerVerifier>::try_from(res)?;

        let verify = verifier
            .verify_server_cert(
                &cert_chain[0],
                &cert_chain[1..],
                server_name,
                &[],
                UnixTime::now(),
            )
            .is_ok();

        let digest: CertDigest = ring::digest::digest(
            &ring::digest::SHA256,
            cert_chain[0].as_ref(),
        )
        .as_ref()
        .try_into()
        .expect("SHA-256 output is always 32 bytes");

        let alpn = NegotiatedAlpn::from_bytes(
            self.writer.get_ref().1.alpn_protocol(),
        );
        let req = CommanderRequest::CheckServerConfigCache(
            self.id, verify, digest, alpn,
        );
        self.commander.send(req).await?;
        let res = recvr
            .recv()
            .await
            .ok_or(StateError::CommanderRecv(COMPLETE_HANDSHAKE))?;
        let recvd_config = Option::<Arc<ServerConfig>>::try_from(res)?;

        let server_config = match recvd_config {
            Some(config) => config,
            None => {
                let owned_cert_chain = cert_chain
                    .iter()
                    .cloned()
                    .map(|x| x.into_owned())
                    .collect();
                let req = CommanderRequest::GetServerConfig(
                    self.id,
                    verify,
                    digest,
                    owned_cert_chain,
                    alpn,
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
            request: self.request,
        })
    }
}
