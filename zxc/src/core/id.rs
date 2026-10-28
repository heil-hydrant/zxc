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
    pub const fn h2_stream(conn_id: usize, stream_id: u32) -> Self {
        Self::H2Stream {
            conn_id,
            stream_id,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soldier_id_connection() {
        let id = SoldierId::connection(42);
        assert_eq!(id.conn_id(), 42);
        assert_eq!(id.stream_id(), None);
        assert!(!id.is_h2());
        assert_eq!(format!("{id}"), "conn:42");
    }

    #[test]
    fn test_soldier_id_h2_stream() {
        let id = SoldierId::h2_stream(42, 7);
        assert_eq!(id.conn_id(), 42);
        assert_eq!(id.stream_id(), Some(7));
        assert!(id.is_h2());
        assert_eq!(format!("{id}"), "conn:42:s:7");
    }

    #[test]
    fn test_soldier_id_equality_and_hashing() {
        use std::collections::HashSet;
        let mut set = HashSet::new();

        let conn_1 = SoldierId::connection(1);
        let stream_1 = SoldierId::h2_stream(1, 1);
        let stream_2 = SoldierId::h2_stream(1, 2);

        assert_ne!(conn_1, stream_1);
        assert_ne!(stream_1, stream_2);

        set.insert(conn_1);
        set.insert(stream_1);
        set.insert(stream_2);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&SoldierId::connection(1)));
        assert!(set.contains(&SoldierId::h2_stream(1, 1)));
    }
}
