//! I/O helpers shared by the proxy and the UI runners.
pub mod file;
pub mod file_attr;
pub mod inc_dir;
pub mod socket;
pub mod unix_sock;

pub use file::*;
pub use inc_dir::*;
pub use socket::*;
use tokio::io::AsyncWriteExt;
pub use unix_sock::*;

pub async fn write_and_flush<T>(
    stream: &mut T,
    buf: &[u8],
) -> Result<(), std::io::Error>
where
    T: AsyncWriteExt + Unpin,
{
    stream.write_all(buf).await?;
    stream.flush().await
}
