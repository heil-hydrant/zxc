use tokio::sync::mpsc::{Receiver, Sender};

use crate::CommanderRequest;
use crate::commander::CommanderResponse;

// Trait to communicate with commander.
//
// Implemented as derive macro:
//     zxc-derive/src/can_communicate

pub trait CanCommunicate {
    fn sender(&mut self) -> &mut Sender<CommanderRequest>;

    fn receiver(&mut self) -> &mut Receiver<CommanderResponse>;
}
