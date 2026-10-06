use std::io;

use rustls_pki_types::ServerName;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::error::SendError;
use tokio_rustls::client::{TlsStream, TlsStream as ClientTlsStream};
use tokio_rustls::{StartHandshake, TlsConnector};

use super::*;
use crate::commander::CommanderResponse;
use crate::commander::captain_crypto::{ALPN_H1, ALPN_H2};
use crate::commander::communicate::response::convert::WrongMessage;
use crate::proxy::states::StateError;

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
