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
            is_h2: conn.is_h2,
            h2_info: conn.h2_info,
        }
    }
}

// Convert SendInfo -> RepeaterConn<ZStream>
impl TryFrom<SendInfo> for RepeaterConn<ZStream> {
    type Error = AddressError;

    fn try_from(info: SendInfo) -> Result<Self, Self::Error> {
        let SendInfo {
            file,
            update,
            server_info,
            is_h2,
            h2_info,
        } = info;
        // default update to true if not set in info.update;
        let update = update.unwrap_or(true);
        let is_h2 = is_h2.unwrap_or(false);
        let path = PathBuf::from(&file);
        let server_info = ServerInfo::try_from(server_info)?;
        Ok(RepeaterConn {
            server_info,
            stream: ZStream,
            path,
            update,
            is_h2,
            h2_info,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forward_info::H2Info;
    use crate::proxy::prelude::*;

    #[test]
    fn test_send_info_to_repeater_conn_carries_h2_metadata() {
        let info = SendInfo {
            file: PathBuf::from("r-1/rep.req"),
            update: Some(false),
            server_info: ServerInfoJson::new(
                "example.com:443".to_string(),
                Scheme::Https,
                None,
            ),
            is_h2: Some(true),
            h2_info: Some(H2Info::new(
                Some("example.com:443".to_string()),
                Some("https".to_string()),
            )),
        };

        let conn = RepeaterConn::<ZStream>::try_from(info).unwrap();

        assert!(conn.is_h2);
        assert!(!conn.update);
        assert_eq!(conn.h2_info.unwrap().authority(), Some("example.com:443"));
    }
}
