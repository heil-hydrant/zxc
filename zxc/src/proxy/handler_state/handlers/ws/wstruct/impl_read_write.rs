use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{
    Message, {self}
};
use tracing::trace;

use super::WsStruct;
use super::error::WsError;
use crate::proxy::handler_state::ProxyState;
use crate::proxy::handler_state::read_write::ReadWrite;

impl<T, E> ReadWrite for WsStruct<T, E>
where
    SplitSink<WebSocketStream<E>, Message>: Sink<Message>,
    SplitStream<WebSocketStream<T>>:
        Stream<Item = Result<Message, tungstenite::Error>>,
{
    type Error = WsError;
    type State = ProxyState<Self>;

    /* Steps:
     *      1. call next() method on stream
     *      2. if next returns a message, set the frame to the message
     *      3. if next returns an error, return the error
     *
     * Transition:
     *      Read -> ShouldLog
     *
     * Error:
     *      WsError::Read [3]
     */

    async fn read(mut self) -> Result<ProxyState<Self>, WsError> {
        if let Some(msg) = self.reader.next().await {
            self.frame = Some(msg?);
        }
        trace!("Y");
        Ok(ProxyState::ShouldLog(self))
    }

    /* Steps:
     *      1. Take the frame
     *      2. Save is_close frame
     *      3. Send the frame
     *      4. If !is_close, return ProxyState::Receive
     *      5. Else return ProxyState::End
     *
     * Error:
     *      WsError::Write [3]
     *
     * Transition:
     *      Write -> End | Receive
     */

    async fn write(mut self) -> Result<ProxyState<Self>, WsError> {
        let data = self.frame.take().unwrap(); // safe to unwrap
        let is_close = data.is_close();
        self.writer
            .send(data)
            .await
            .map_err(|_| WsError::Write)?;
        trace!("Y");
        if !is_close {
            trace!("continue");
            return Ok(ProxyState::Receive(self));
        }
        trace!("close frame");
        Ok(ProxyState::End(self))
    }
}
