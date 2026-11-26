use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::async_step::async_run;
use crate::io::*;
use crate::proxy::*;
use crate::repeater::prelude::*;

use super::connection::Rh2;

// One request/response per connection. The H2 connection driver and the
// response future are polled directly by the repeater state machine.
pub async fn handle_h2<T>(
    conn: RepeaterConn<T>,
) -> Result<Rh2<T>, ProtocolError>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&conn.path)
        .await
        .map_err(|e| FileErrorInfo::open_fail(&conn.path, e))?;
    let h2: Rh2<T> = Rh2::from((conn, file));
    let state = RepeaterState::ReadFromFile(h2);
    match async_run(state).await? {
        RepeaterState::End(hconn) => Ok(hconn),
        _ => {
            unreachable!();
        }
    }
}
