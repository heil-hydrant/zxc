use std::fmt::{Debug, Display, Formatter};
use std::marker::Unpin;

use connection::encrypt::server_encrypt;
use header_plz::OneRequestLine;
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional_with_sizes};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tokio_rustls::StartHandshake;
pub use tokio_rustls::client::TlsStream as ClientTlsStream;
pub use tokio_rustls::server::TlsStream as ServerTlsStream;
use tracing::trace;

use crate::async_step::AsyncStep;
use crate::commander::{CommanderResponse, Protocol};
use crate::io::socket::establish_connection;
use crate::io::write::write_and_flush;
use crate::proxy::server_info::ServerInfo;
use crate::proxy::server_info::address::get_address;
use crate::{CAPACITY_2MB, CommanderRequest};
pub mod connection;
pub use connection::{Connection, ZStream};
pub mod error;
use error::*;

use super::handler_state::ProxyState;
use super::handler_state::additional_handler_info::AdditionalHandlerInfo;
use super::handler_state::handlers::oneonestruct::OneOneStruct;
use super::handler_state::handlers::{handle_http, read_http};

pub const PROXY_ESTABLISHED: &[u8; 39] =
    b"HTTP/1.1 200 Connection established\r\n\r\n";

// type alias
pub type Tcp = TcpStream;

pub enum ConnectionState<T> {
    ReadInitialClientData(Connection<T, ZStream>),
    DetermineEncryption(Connection<T, ZStream>),
    DetermineServer(Connection<T, ZStream>, bool),
    EstablishServerConnection(Connection<T, ZStream>, ServerInfo),
    ShouldProxy(Connection<T, Tcp>, ServerInfo),
    Relay(Connection<T, Tcp>, ServerInfo),
    ClientHandShake(
        Connection<T, Tcp>,
        Receiver<CommanderResponse>,
        ServerInfo,
    ),
    EncryptServer(
        Connection<StartHandshake<T>, Tcp>,
        Receiver<CommanderResponse>,
        ServerInfo,
    ),
    CompleteHandshake(
        Connection<StartHandshake<T>, ClientTlsStream<Tcp>>,
        Receiver<CommanderResponse>,
        ServerInfo,
    ),
    HandleTls(
        Connection<ServerTlsStream<T>, ClientTlsStream<Tcp>>,
        Receiver<CommanderResponse>,
        ServerInfo,
        Protocol,
    ),
    HandleTcp(
        Connection<T, Tcp>,
        Receiver<CommanderResponse>,
        ServerInfo,
        Protocol,
    ),
    EstablishTlsTcp(
        Connection<ServerTlsStream<T>, ClientTlsStream<Tcp>>,
        AdditionalHandlerInfo,
    ),
    HandleTlsTcp(Connection<ServerTlsStream<T>, Tcp>, AdditionalHandlerInfo),
    EstablishTcpTls(Connection<T, Tcp>, AdditionalHandlerInfo),
    HandleTcpTls(Connection<T, ClientTlsStream<Tcp>>, AdditionalHandlerInfo),
    End,
}

impl<T> ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    pub fn new(
        id: usize,
        client: T,
        tx: Sender<CommanderRequest>,
    ) -> ConnectionState<T> {
        Self::ReadInitialClientData(Connection::<T, ZStream>::new(
            id, client, tx,
        ))
    }
}

impl<T> AsyncStep for ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin + Sync + Send + 'static + Debug,
{
    type Error = StateError;

    async fn next(self) -> Result<Self, StateError> {
        match self {
            Self::ReadInitialClientData(mut conn) => {
                let request = read_http::<T, OneRequestLine>(
                    &mut conn.reader,
                    &mut conn.buf,
                )
                .await
                .map_err(StateError::InitialRead)?;
                conn.request = Some(request);
                Ok(Self::DetermineEncryption(conn))
            }
            Self::DetermineEncryption(conn) => {
                // Safe to unwrap request as it has been set in previous state
                let tls = conn
                    .request
                    .as_ref()
                    .unwrap()
                    .is_connect_request();
                Ok(Self::DetermineServer(conn, tls))
            }

            Self::DetermineServer(mut conn, tls) => {
                // Safe to unwrap request as it has been set in Read Initial
                // Client Data
                let addr = get_address(
                    conn.request
                        .as_mut()
                        .unwrap()
                        .message_head_mut()
                        .info_line_mut(),
                    tls,
                )?;
                let server_info = ServerInfo::new(addr, tls, None);
                trace!("{}", server_info);
                Ok(Self::EstablishServerConnection(conn, server_info))
            }

            Self::EstablishServerConnection(conn, server_info) => {
                let stream =
                    establish_connection(server_info.address()).await?;
                let conn = Connection::from((conn, stream));
                Ok(Self::ShouldProxy(conn, server_info))
            }

            Self::ShouldProxy(conn, server_info) => {
                let (tx, rx) = oneshot::channel();
                let request = CommanderRequest::ShouldProxy(
                    conn.id,
                    server_info.address().to_string(),
                    tx,
                );
                conn.commander.send(request).await?;

                match rx.await? {
                    Some(recvr) => {
                        if server_info.is_tls() {
                            Ok(Self::ClientHandShake(conn, recvr, server_info))
                        } else {
                            Ok(Self::HandleTcp(
                                conn,
                                recvr,
                                server_info,
                                Protocol::OneOne,
                            ))
                        }
                    }
                    _ => Ok(Self::Relay(conn, server_info)),
                }
            }

            Self::Relay(mut conn, server_info) => {
                if server_info.is_tls() {
                    write_and_flush(&mut conn.reader, PROXY_ESTABLISHED)
                        .await
                        .map_err(StateError::ClientWrite)
                } else {
                    let request = conn.request.take().unwrap();
                    let mut chained = request.as_chain();
                    conn.writer
                        .write_all_buf(&mut chained)
                        .await
                        .map_err(StateError::ServerWrite)
                }?;
                let _ = copy_bidirectional_with_sizes(
                    &mut conn.reader,
                    &mut conn.writer,
                    CAPACITY_2MB,
                    CAPACITY_2MB,
                )
                .await;
                Ok(Self::End)
            }

            Self::ClientHandShake(mut conn, recvr, server_info) => {
                write_and_flush(&mut conn.reader, PROXY_ESTABLISHED)
                    .await
                    .map_err(StateError::ClientWrite)?;
                let conn = conn.perform_handshake().await?;
                Ok(Self::EncryptServer(conn, recvr, server_info))
            }

            Self::EncryptServer(conn, mut recvr, mut server_info) => {
                let conn = conn
                    .encrypt_server(&mut recvr, &mut server_info)
                    .await?;
                Ok(Self::CompleteHandshake(conn, recvr, server_info))
            }

            Self::CompleteHandshake(conn, mut recvr, server_info) => {
                let conn = conn
                    .complete_handshake(&mut recvr, &server_info)
                    .await?;
                Ok(Self::HandleTls(conn, recvr, server_info, Protocol::OneOne))
            }

            Self::HandleTls(conn, recvr, server_info, _protocol) => {
                let mut client = OneOneStruct::<_, _, OneRequestLine>::from((
                    conn,
                    recvr,
                    server_info,
                ));
                client.frame.take();
                let client_state = ProxyState::Receive(client);
                handle_http(client_state).await
            }

            Self::HandleTcp(conn, recvr, server_info, _protocol) => {
                let client = OneOneStruct::<_, _, OneRequestLine>::from((
                    conn,
                    recvr,
                    server_info,
                ));
                let client_state = ProxyState::ShouldLog(client);
                handle_http(client_state).await
            }

            Self::EstablishTlsTcp(conn, addinfo) => {
                let tcp = establish_connection(addinfo.address()).await?;
                let conn = Connection::from((conn, tcp));
                Ok(Self::HandleTlsTcp(conn, addinfo))
            }

            Self::HandleTlsTcp(conn, addinfo) => {
                let oneone = OneOneStruct::<_, _, OneRequestLine>::from((
                    conn, addinfo,
                ));
                let client_state = ProxyState::Send(oneone);
                handle_http(client_state).await
            }

            Self::EstablishTcpTls(mut conn, mut addinfo) => {
                let tcp = establish_connection(addinfo.address()).await?;
                let sni = addinfo.sni().to_owned();
                let tls = server_encrypt(
                    conn.id,
                    &mut conn.commander,
                    &mut addinfo.receiver,
                    sni,
                    tcp,
                )
                .await?;
                let conn = Connection::from((conn, tls));
                Ok(Self::HandleTcpTls(conn, addinfo))
            }

            Self::HandleTcpTls(conn, addinfo) => {
                let oneone = OneOneStruct::<_, _, OneRequestLine>::from((
                    conn, addinfo,
                ));
                let client_state = ProxyState::Send(oneone);
                handle_http(client_state).await
            }
            Self::End => Ok(Self::End),
        }
    }

    fn is_ended(&self) -> bool {
        matches!(self, Self::End)
    }
}

impl<T> Display for ConnectionState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadInitialClientData(_) => write!(f, "initial_read"),
            Self::DetermineEncryption(_) => write!(f, "encryption"),
            Self::DetermineServer(..) => write!(f, "server"),
            Self::EstablishServerConnection(..) => write!(f, "server conn"),
            Self::ShouldProxy(..) => write!(f, "should_proxy"),
            Self::Relay(..) => write!(f, "relay"),
            Self::ClientHandShake(..) => write!(f, "client_handshake"),
            Self::EncryptServer(..) => write!(f, "server_encrypt"),
            Self::CompleteHandshake(..) => write!(f, "client_encrypt"),
            Self::HandleTls(_, _, info, _)
            | Self::HandleTcp(_, _, info, _) => write!(f, "{info}"),
            Self::HandleTlsTcp(_, addinfo)
            | Self::HandleTcpTls(_, addinfo) => {
                write!(f, "{}", addinfo.server_info)
            }
            Self::EstablishTlsTcp(_, addinfo) => {
                write!(f, "establish_tls_tcp| {}", addinfo.server_info)
            }
            Self::EstablishTcpTls(_, addinfo) => {
                write!(f, "establish_tcp_tls| {}", addinfo.server_info)
            }
            Self::End => Ok(()),
        }
    }
}
