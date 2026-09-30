use super::*;

//  Connection + Receiver<CommanderResponse> + ServerInfo =>
//          OneOneStruct<OneRequestLine>

impl<T, E> From<(Connection<T, E>, Receiver<CommanderResponse>, ServerInfo)>
    for OneOneStruct<T, E, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(
        (conn, commander_recvr, server_info): (
            Connection<T, E>,
            Receiver<CommanderResponse>,
            ServerInfo,
        ),
    ) -> Self {
        Self {
            server_info,
            buf: conn.buf,
            commander_sendr: conn.commander,
            commander_recvr,
            payload: None,
            file: None,
            frame: conn.request,
            id: conn.id,
            path: None,
            reader: conn.reader,
            role: Role::Server,
            need_response: false,
            writer: conn.writer,
            log_id: 0,
            history_sendr: None,
        }
    }
}
