use super::*;

/* OneOneStruct<Response> => OneOneStruct<Request>
 *
 * Used:
 *      When Reading from server failed,
 *      Since request file is already stored in Response struct reuse it.
 */

impl<T, E> From<OneOneStruct<T, E, OneResponseLine>>
    for OneOneStruct<E, T, OneRequestLine>
{
    fn from(mut response: OneOneStruct<T, E, OneResponseLine>) -> Self {
        response.buf.clear();
        Self {
            buf: response.buf,
            commander_sendr: response.commander_sendr,
            commander_recvr: response.commander_recvr,
            log_id: response.log_id,
            payload: None,
            file: response.file, // .req file reused in reconnect
            frame: None,
            id: response.id,
            path: response.path,
            reader: response.writer,
            role: Role::Server,
            need_response: response.need_response,
            writer: response.reader,
            server_info: response.server_info,
            history_sendr: response.history_sendr,
        }
    }
}
