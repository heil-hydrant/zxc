use crate::interceptor::{InterToUI, InterceptorHandler};
use crate::run::boundary::FromCommander;

impl FromCommander for InterceptorHandler {
    type Message = InterToUI;

    async fn recv(&mut self) -> Option<Self::Message> {
        self.from_commander.recv().await
    }
}
