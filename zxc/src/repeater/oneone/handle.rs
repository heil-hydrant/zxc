use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::async_step::async_run;
use crate::io::*;
use crate::proxy::*;
use crate::repeater::prelude::*;

use super::connection::Roneone;

pub async fn handle_http<T>(
    conn: RepeaterConn<T>,
) -> Result<Roneone<T>, ProtocolError>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin,
{
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&conn.path)
        .await
        .map_err(|e| FileErrorInfo::open_fail(&conn.path, e))?;
    let one: Roneone<T> = Roneone::from((conn, file));
    let state = RepeaterState::ReadFromFile(one);
    match async_run(state).await? {
        RepeaterState::End(hconn) => Ok(hconn),
        _ => {
            unreachable!();
        }
    }
}
