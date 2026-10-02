use super::*;
use crate::proxy::states::{
    ClientTlsStream, ConnectionState, ServerTlsStream, Tcp
};

/* OneOneStruct<T,Tcp,OneRequestLine> => ConnectionState<T>
 *
 * Used in:
 *      ProxyState::NewConnection
 *      when,
 *          client      = tcp://
 *          server      = tcp://
 *          new server  = tls://
 */

impl<T> From<OneOneStruct<T, Tcp, OneRequestLine>> for ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(oneone: OneOneStruct<T, Tcp, OneRequestLine>) -> Self {
        let (conn, addinfo) = oneone.into();
        ConnectionState::EstablishTcpTls(conn, addinfo)
    }
}

/* OneOneStruct<ServerTlsStream<T>,ClientTlsStream<Tcp>,OneRequestLine> =>
 *          ConnectionState<T>
 *
 * Used in:
 *      ProxyState::NewConnection
 *      when,
 *          client      = tls://
 *          server      = tls://
 *          new server  = tcp://
 */

impl<T>
    From<
        OneOneStruct<ServerTlsStream<T>, ClientTlsStream<Tcp>, OneRequestLine>,
    > for ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(
        oneone: OneOneStruct<
            ServerTlsStream<T>,
            ClientTlsStream<Tcp>,
            OneRequestLine,
        >,
    ) -> Self {
        let (conn, addinfo) = oneone.into();
        ConnectionState::EstablishTlsTcp(conn, addinfo)
    }
}

/* OneOneStruct<ServerTlsStream<T>,Tcp,OneRequestLine> => ConnectionState<T>
 *
 *  Blank implementation
 *
 *  Should not reach this state, can_reconnect() should succeed in reconnect
 *  state
 */

impl<T> From<OneOneStruct<ServerTlsStream<T>, Tcp, OneRequestLine>>
    for ConnectionState<T>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(_: OneOneStruct<ServerTlsStream<T>, Tcp, OneRequestLine>) -> Self {
        unreachable!();
    }
}

/* OneOneStruct<T,ClientTlsStream<Tcp>,OneRequestLine> => ConnectionState<T>
 *
 *  Blank implementation
 *
 *  Should not reach this state, can_reconnect() should succeed in reconnect
 *  state
 */

impl<T> From<OneOneStruct<T, ClientTlsStream<Tcp>, OneRequestLine>>
    for ConnectionState<T>
{
    fn from(_: OneOneStruct<T, ClientTlsStream<Tcp>, OneRequestLine>) -> Self {
        unreachable!();
    }
}
