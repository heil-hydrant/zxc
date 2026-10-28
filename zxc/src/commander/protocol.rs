use super::Alpn;

#[derive(Debug)]
pub enum Protocol {
    OneOne,
    Websocket,
    Two,
}

impl From<Alpn> for Protocol {
    fn from(value: Alpn) -> Self {
        match value {
            Alpn::None | Alpn::Http1 => Protocol::OneOne,
            Alpn::H2 => Protocol::Two,
        }
    }
}
