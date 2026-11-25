use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SoldierId {
    /// HTTP/1.1 or WebSocket connection-level soldier
    Connection(usize),
    /// HTTP/2 multiplexed stream soldier
    H2Stream {
        conn_id: usize,
        stream_id: u32,
    },
}

impl SoldierId {
    #[inline(always)]
    pub const fn connection(conn_id: usize) -> Self {
        Self::Connection(conn_id)
    }

    #[inline(always)]
    pub fn conn_id(&self) -> usize {
        match *self {
            Self::Connection(id)
            | Self::H2Stream {
                conn_id: id,
                ..
            } => id,
        }
    }

    #[inline(always)]
    pub fn stream_id(&self) -> Option<u32> {
        match *self {
            Self::Connection(_) => None,
            Self::H2Stream {
                stream_id,
                ..
            } => Some(stream_id),
        }
    }

    #[inline(always)]
    pub fn is_h2(&self) -> bool {
        matches!(self, Self::H2Stream { .. })
    }
}

impl From<usize> for SoldierId {
    #[inline(always)]
    fn from(conn_id: usize) -> Self {
        Self::Connection(conn_id)
    }
}

impl fmt::Display for SoldierId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(id) => write!(f, "conn:{id}"),
            Self::H2Stream {
                conn_id,
                stream_id,
            } => write!(f, "conn:{conn_id}:s:{stream_id}"),
        }
    }
}

pub trait Id {
    fn id(&self) -> SoldierId;
}
