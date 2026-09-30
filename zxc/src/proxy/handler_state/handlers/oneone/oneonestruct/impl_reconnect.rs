use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::trace;

use super::*;
use crate::io::socket::establish_connection;
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::handler_state::transition::reconnect::Reconnect;
use crate::proxy::server_info::ServerInfo;
use crate::proxy::states::ClientTlsStream;
use crate::proxy::states::connection::encrypt::server_encrypt;

impl<T> Reconnect for OneOneStruct<T, TcpStream, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn can_reconnect(&self, server_info: &ServerInfo) -> bool {
        !server_info.is_tls()
    }

    async fn reconnect(&mut self) -> Result<(), ProxyStateError> {
        self.writer = establish_connection(self.address()).await?;
        Ok(())
    }
}

impl<T> Reconnect
    for OneOneStruct<T, ClientTlsStream<TcpStream>, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn can_reconnect(&self, server_info: &ServerInfo) -> bool {
        server_info.is_tls()
    }

    async fn reconnect(&mut self) -> Result<(), ProxyStateError> {
        let tcp = establish_connection(self.address()).await?;
        trace!("Reconnected");
        let server_name = self.server_info.sni().clone();
        let tls = server_encrypt(
            self.id,
            &mut self.commander_sendr,
            &mut self.commander_recvr,
            server_name,
            tcp,
        )
        .await?;
        trace!("Encrypted");
        self.writer = tls;
        Ok(())
    }
}
