use crate::proxy::prelude::*;

/* Description:
 *      Trait to reconnect to server.
 *      http only
 *
 *
 * Methods:
 *      can_reconnect   :   can be reconnected to server,
 *                          If user defines different scheme to connect to
 *                          original or different server after intercepting,
 *                          return false.
 *
 *      reconnect       :   reconnect to server
 *
 *
 *
 * Implemented in:
 *      oneonestruct/impl_reconnect
 */

pub trait Reconnect {
    fn can_reconnect(&self, server_info: &ServerInfo) -> bool;

    async fn reconnect(&mut self) -> Result<(), ProxyStateError>;
}
