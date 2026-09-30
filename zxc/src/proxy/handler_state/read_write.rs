use super::error::ProxyStateError;

// Trait to read and write data from stream based on the protocol.
pub trait ReadWrite
where
    Self: Sized,
{
    type Error: Into<ProxyStateError>;

    type State;

    async fn read(self) -> Result<Self::State, Self::Error>;

    async fn write(self) -> Result<Self::State, Self::Error>;
}
