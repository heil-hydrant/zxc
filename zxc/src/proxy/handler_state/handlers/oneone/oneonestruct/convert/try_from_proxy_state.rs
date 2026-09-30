use super::*;
use crate::proxy::handler_state::ProxyState;
use crate::proxy::handler_state::handlers::oneone::error::HandleOneOneError;

/* ProxyState<OneOneStruct<T, E, OneRequestLine>> => OneOneStruct<T,E,OneRequestLine>.
 *
 *      i.e. client_state to client_handler
 *
 * Steps:
 *      1. If End, Ok(client_handler)
 *
 *      2. If ServerClose,
 *              HandleOneOneError::SendToServer(client_handler, e)
 *
 *      3. If NewConnection,
 *              HandleOneOneError::NeedNewConnection(client_handler, addinfo)
 *
 *      4. Else, unreachable
 */

impl<T, E> TryFrom<ProxyState<OneOneStruct<T, E, OneRequestLine>>>
    for OneOneStruct<T, E, OneRequestLine>
{
    type Error = HandleOneOneError<T, E>;

    fn try_from(
        client_state: ProxyState<OneOneStruct<T, E, OneRequestLine>>,
    ) -> Result<Self, Self::Error> {
        match client_state {
            ProxyState::End(conn) => Ok(conn),
            ProxyState::ServerClose(conn, e) => {
                Err(HandleOneOneError::SendToServer(conn, e))
            }
            ProxyState::NewConnection(conn, addinfo) => {
                Err(HandleOneOneError::NeedNewConnection(conn, addinfo))
            }
            _ => unreachable!(),
        }
    }
}

/* ProxyState<OneOneStruct<T, E, OneResponseLine>> => OneOneStruct<T,E,OneResponseLine>.
 *
 *      i.e. server_state to server_handler
 *
 * Steps:
 *      1. If End, Ok(server_handler)
 *
 *      2. If ServerClose,
 *              HandleOneOneError::ReadFromServer(server_handler, e)
 *
 *      3. Else, unreachable
 */

impl<T, E> TryFrom<ProxyState<OneOneStruct<T, E, OneResponseLine>>>
    for OneOneStruct<T, E, OneResponseLine>
{
    type Error = HandleOneOneError<E, T>;

    fn try_from(
        server_state: ProxyState<OneOneStruct<T, E, OneResponseLine>>,
    ) -> Result<Self, Self::Error> {
        match server_state {
            ProxyState::End(conn) => Ok(conn),
            ProxyState::ServerClose(conn, e) => {
                Err(HandleOneOneError::ReadFromServer(conn, e))
            }
            _ => unreachable!(),
        }
    }
}
