use std::io;
use std::sync::Arc;

use rustls_pki_types::{ServerName, UnixTime};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};
pub use tokio_rustls::client::TlsStream as ClientTlsStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::client::WebPkiServerVerifier;
use tokio_rustls::rustls::client::danger::ServerCertVerifier;
use tokio_rustls::rustls::server::Acceptor;
pub use tokio_rustls::server::TlsStream as ServerTlsStream;
use tokio_rustls::{LazyConfigAcceptor, StartHandshake, TlsConnector};

use crate::commander::*;
use crate::proxy::prelude::*;

const COMPLETE_HANDSHAKE: &str = "Complete Handshake";

impl<T, E> Connection<T, E>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin, // Client Stream
{
    pub async fn perform_handshake(
        self,
    ) -> Result<Connection<StartHandshake<T>, E>, StateError> {
        let handshake =
            LazyConfigAcceptor::new(Acceptor::default(), self.reader)
                .await
                .map_err(StateError::ClientHandshake)?;
        Ok(Connection {
            id: self.id,
            commander: self.commander,
            reader: handshake,
            writer: self.writer,
            buf: self.buf,
            request: self.request,
        })
    }
}

impl<T> Connection<StartHandshake<T>, ClientTlsStream<TcpStream>>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    pub async fn complete_handshake(
        self,
        recvr: &mut Receiver<CommanderResponse>,
        server_info: &ServerInfo,
    ) -> Result<
        (Connection<ServerTlsStream<T>, ClientTlsStream<TcpStream>>, Alpn),
        StateError,
    > {
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

        let alpn = Alpn::from_bytes(self.writer.get_ref().1.alpn_protocol());
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
        Ok((
            Connection {
                id: self.id,
                commander: self.commander,
                reader: stream,
                writer: self.writer,
                buf: self.buf,
                request: self.request,
            },
            alpn,
        ))
    }
}

impl<T> Connection<StartHandshake<T>, TcpStream>
where
    T: AsyncReadExt + AsyncWriteExt + std::marker::Unpin,
{
    pub async fn encrypt_server(
        mut self,
        recvr: &mut Receiver<CommanderResponse>,
        server_info: &mut ServerInfo,
    ) -> Result<
        Connection<StartHandshake<T>, ClientTlsStream<TcpStream>>,
        StateError,
    > {
        let client_hello = self.reader.client_hello();
        let sni = client_hello.server_name();
        let only_h1 = client_hello
            .alpn()
            .map(|mut protos| !protos.any(|p| p == ALPN_H2))
            .unwrap_or(true);
        let server_name: ServerName = server_info
            .address()
            .parse_sni(sni)?
            .to_owned();
        let stream = server_encrypt(
            self.id,
            &mut self.commander,
            recvr,
            server_name.clone(),
            self.writer,
            only_h1,
        )
        .await?;

        server_info.set_sni(server_name);
        Ok(Connection {
            id: self.id,
            commander: self.commander,
            reader: self.reader,
            writer: stream,
            buf: self.buf,
            request: self.request,
        })
    }
}

#[derive(Debug, Error)]
pub enum ServerEncryptError {
    #[error("commander send")]
    Send(#[from] SendError<CommanderRequest>),
    #[error("commander recv")]
    Recv,
    #[error("wrong msg| {0}")]
    WrongMessage(#[from] WrongMessage),
    #[error("io| {0}")]
    Io(#[from] io::Error),
}

pub async fn server_encrypt(
    id: usize,
    sender: &mut Sender<CommanderRequest>,
    recvr: &mut Receiver<CommanderResponse>,
    server_name: ServerName<'static>,
    stream: TcpStream,
    only_h1: bool,
) -> Result<TlsStream<TcpStream>, ServerEncryptError> {
    let req = CommanderRequest::GetClientConnector(id);
    sender.send(req).await?;
    let res = recvr
        .recv()
        .await
        .ok_or(ServerEncryptError::Recv)?;
    let connector = Arc::<TlsConnector>::try_from(res)?;
    if only_h1 {
        connector
            .with_alpn(vec![ALPN_H1.to_vec()])
            .connect(server_name, stream)
    } else {
        connector.connect(server_name, stream)
    }
    .await
    .map_err(Into::into)
}
