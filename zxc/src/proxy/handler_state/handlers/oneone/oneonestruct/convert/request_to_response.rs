use super::*;

// OneOneStruct<Request> to OneOneStruct<Response>

impl<T, E> From<OneOneStruct<T, E, OneRequestLine>>
    for OneOneStruct<E, T, OneResponseLine>
{
    fn from(request: OneOneStruct<T, E, OneRequestLine>) -> Self {
        Self {
            buf: request.buf,
            commander_sendr: request.commander_sendr,
            commander_recvr: request.commander_recvr,
            log_id: request.log_id,
            payload: None,
            file: request.file, // Can be reused in reconnect
            frame: None,
            id: request.id,
            path: request.path,
            reader: request.writer,
            role: Role::Client,
            need_response: request.need_response,
            writer: request.reader,
            server_info: request.server_info,
            history_sendr: request.history_sendr,
        }
    }
}
