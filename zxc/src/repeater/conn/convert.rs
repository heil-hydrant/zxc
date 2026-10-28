use std::path::PathBuf;

use tokio::net::TcpStream;

use crate::proxy::server_info::address::error::AddressError;
use crate::proxy::*;
use crate::repeater::prelude::*;

// Convert RepeaterConn<ZStream> -> RepeaterConn<TcpStream>
impl From<(RepeaterConn<ZStream>, TcpStream)> for RepeaterConn<TcpStream> {
    fn from((conn, stream): (RepeaterConn<ZStream>, TcpStream)) -> Self {
        RepeaterConn {
            path: conn.path,
            stream,
            server_info: conn.server_info,
            update: conn.update,
        }
    }
}

// Convert SendInfo -> RepeaterConn<ZStream>
impl TryFrom<SendInfo> for RepeaterConn<ZStream> {
    type Error = AddressError;

    fn try_from(info: SendInfo) -> Result<Self, Self::Error> {
        // default update to true if not set in info.update;
        let update = info.update.unwrap_or(true);
        let path = PathBuf::from(&info.file);
        let server_info = ServerInfo::try_from(info.server_info)?;
        Ok(RepeaterConn {
            server_info,
            stream: ZStream,
            path,
            update,
        })
    }
}
