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
use crate::commander::communicate::response::convert::WrongMessage;
use crate::proxy::states::StateError;

/* Description:
 *      Performs Server Stream Encyption.
 *
 * Steps:
 *      1. Get client hello by calling client_hello()
 *      2. Get sni from client_hello by calling server_name()
 *      3. Get ServerName by passing sni to server_info.address.get_servername()
 *      4. Encrypt server stream by calling server_encrypt().
 *      5. Store the ServerName in self.server_name .
 *
 * Error:
 *      StateError::InvalidDns      [3]
 *      StateError::ServerEncrypt   [4]
 */

impl<T> Connection<StartHandshake<T>, TcpStream>
where
    T: AsyncReadExt + AsyncWriteExt + std::marker::Unpin, // Stream
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

/* Description:
 *      Function to encrypt server stream.
 *
 * Steps:
 *      1. Build Communicate::GetClientConfig request.
 *      2. Send request and receive response
 *      3. Get Arc<TlsConnector> from response [TryFrom trait implemented in
 *         response/convert.rs ]
 *      4. Encrypt server by calling connect() with args ServerName and server
 *         stream on tls_connector received from commander.
 *
 * Error:
 *      ServerEncryptError::Send            [2]
 *      ServerEncryptError::Recv            [2]
 *      ServerEncryptError::WrongMessage    [3]
 *      ServerEncryptError::Io              [4]
 */

pub async fn server_encrypt(
    id: usize,
    sender: &mut Sender<CommanderRequest>,
    recvr: &mut Receiver<CommanderResponse>,
    server_name: ServerName<'static>,
    stream: TcpStream,
) -> Result<TlsStream<TcpStream>, ServerEncryptError> {
    let req = CommanderRequest::GetClientConfig(id);
    sender.send(req).await?;
    let res = recvr
        .recv()
        .await
        .ok_or(ServerEncryptError::Recv)?;
    let connector = Arc::<TlsConnector>::try_from(res)?;
    connector
        .connect(server_name.clone(), stream)
        .await
        .map_err(Into::into)
}
