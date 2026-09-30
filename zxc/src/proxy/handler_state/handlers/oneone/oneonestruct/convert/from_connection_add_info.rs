use header_plz::OneRequestLine;

use super::*;
use crate::proxy::handler_state::additional_handler_info::AdditionalHandlerInfo;

/* Connection + AdditionalHandlerInfo => OneOneStruct<Request>
 *
 * Used in:
 *          State::SwitchTlsTcp
 *          State::SwitchTcpTls
 */

impl<T, E> From<(Connection<T, E>, AdditionalHandlerInfo)>
    for OneOneStruct<T, E, OneRequestLine>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
    E: AsyncReadExt + AsyncWriteExt + Unpin,
{
    fn from(
        (conn, addinfo): (Connection<T, E>, AdditionalHandlerInfo),
    ) -> Self {
        let mut one = OneOneStruct::<T, E, OneRequestLine>::from((
            conn,
            addinfo.receiver,
            addinfo.server_info,
        ));
        one.log_id = addinfo.log_id;
        one.path = Some(addinfo.path);
        one.payload = Some(addinfo.payload);
        one.need_response = addinfo.should_intercept;
        one.history_sendr = addinfo.history_sender;
        one
    }
}
