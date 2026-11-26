mod convert;
pub mod error;

use std::fmt::Display;
use std::net::SocketAddr;

pub use error::AddressError;
use header_plz::OneRequestLine;
use rustls_pki_types::{InvalidDnsNameError, ServerName};

use super::scheme::Scheme;

// Enum to represent the different address types of the server.
// The DNS variant holds host and port as a tuple.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Address {
    Socket(SocketAddr),
    Dns((String, u16)),
}

impl Address {
    pub fn port(&self) -> u16 {
        match self {
            Address::Socket(socket_addr) => socket_addr.port(),
            Address::Dns((_, port)) => *port,
        }
    }

    pub fn set_port(&mut self, port: u16) {
        match self {
            Address::Socket(socket_addr) => {
                *socket_addr = SocketAddr::new(socket_addr.ip(), port)
            }
            Address::Dns((_, old_port)) => *old_port = port,
        }
    }

    pub fn parse_sni<'a>(
        &'a self,
        sni: Option<&'a str>,
    ) -> Result<ServerName<'a>, InvalidDnsNameError> {
        if let Some(sni) = sni {
            return ServerName::try_from(sni);
        }
        match self {
            Address::Socket(socket_addr) => {
                Ok(ServerName::from(socket_addr.ip()))
            }
            Address::Dns((host, _)) => ServerName::try_from(host.as_str()),
        }
    }

    // `rustls-pki-types` `ServerName` uses the no-std `IpAddr`, so
    // convert it to `String` for comparison.
    // https://docs.rs/rustls-pki-types/latest/rustls_pki_types/enum.IpAddr.html
    pub fn is_host_sni_equal(&self, sni: &ServerName) -> bool {
        let sni = sni.to_str();
        match self {
            Address::Socket(addr) => addr.ip().to_string() == sni.as_ref(),
            Address::Dns((dns, _)) => dns.eq_ignore_ascii_case(sni.as_ref()),
        }
    }

    pub fn to_string_from_scheme(&self, scheme: Scheme) -> String {
        if self.port() == scheme.default_port() {
            return match self {
                Address::Socket(socket_addr) => socket_addr.ip().to_string(),
                Address::Dns((host, _)) => host.to_string(),
            };
        }
        self.to_string()
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Socket(socket_addr) => write!(f, "{}", socket_addr),
            Address::Dns((host, port)) => write!(f, "{}:{}", host, port),
        }
    }
}

pub fn get_address(
    request_line: &mut OneRequestLine,
    tls: bool,
) -> Result<Address, AddressError> {
    if tls {
        Address::try_from(request_line.uri_as_ref())
    } else {
        let uri = request_line.uri()?;
        let path = uri.path_and_query().as_str();
        request_line.set_uri(path.as_ref());
        let mut address = Address::try_from(
            uri.authority()
                .ok_or(AddressError::EmptyHost)?,
        )?;
        if address.port() == 0 {
            address.set_port(80)
        }
        Ok(address)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bytes::BytesMut;
    use http_plz::OneRequest;

    use super::*;

    fn build_request(raw: &str) -> OneRequest {
        OneRequest::try_from(BytesMut::from(raw)).unwrap()
    }

    #[test]
    fn test_address_set_port() {
        let mut address = Address::try_from("127.0.0.1:9001").unwrap();
        address.set_port(8080);
        assert_eq!(
            address,
            Address::Socket(SocketAddr::from_str("127.0.0.1:8080").unwrap())
        )
    }

    #[test]
    fn test_address_from_request_http_ip_get() {
        let mut request =
            build_request("GET http://127.0.0.1:8080/echo HTTP/1.1\r\n\r\n");
        let address =
            get_address(request.message_head_mut().info_line_mut(), false)
                .unwrap();
        assert_eq!(
            address,
            Address::Socket(SocketAddr::from_str("127.0.0.1:8080").unwrap())
        );
        assert_eq!(
            request.into_bytes(),
            BytesMut::from("GET /echo HTTP/1.1\r\n\r\n")
        );
    }

    #[test]
    fn test_address_from_request_http_ip_post() {
        let mut request =
            build_request("POST http://127.0.0.1:8080/echo HTTP/1.1\r\n\r\n");
        let address =
            get_address(request.message_head_mut().info_line_mut(), false)
                .unwrap();
        assert_eq!(
            address,
            Address::Socket(SocketAddr::from_str("127.0.0.1:8080").unwrap())
        );
        assert_eq!(
            request.into_bytes(),
            BytesMut::from("POST /echo HTTP/1.1\r\n\r\n")
        );
    }

    #[test]
    fn test_address_from_request_https_ip() {
        let mut request =
            build_request("CONNECT 127.0.0.1:8080 HTTP/1.1\r\n\r\n");
        let address =
            get_address(request.message_head_mut().info_line_mut(), true)
                .unwrap();
        assert_eq!(
            address,
            Address::Socket(SocketAddr::from_str("127.0.0.1:8080").unwrap())
        )
    }

    #[test]
    fn test_address_from_request_http_dns() {
        let mut request =
            build_request("GET http://www.google.com/echo HTTP/1.1\r\n\r\n");
        let address =
            get_address(request.message_head_mut().info_line_mut(), false)
                .unwrap();
        assert_eq!(address, Address::Dns(("www.google.com".to_string(), 80)))
    }

    #[test]
    fn test_address_from_request_https_dns() {
        let mut request =
            build_request("CONNECT www.google.com:443 HTTP/1.1\r\n\r\n");
        let address =
            get_address(request.message_head_mut().info_line_mut(), true)
                .unwrap();
        assert_eq!(address, Address::Dns(("www.google.com".to_string(), 443)))
    }

    #[test]
    fn test_host_sni_equal_ip() {
        let server_name = ServerName::try_from("127.0.0.1").unwrap();
        let address = Address::Socket("127.0.0.1:80".parse().unwrap());
        assert!(address.is_host_sni_equal(&server_name));
    }

    #[test]
    fn test_host_sni_equal_dns() {
        let server_name = ServerName::try_from("www.google.com").unwrap();
        let address = Address::Dns(("www.google.com".to_string(), 80));
        assert!(address.is_host_sni_equal(&server_name));
    }

    #[test]
    fn test_host_sni_equal_dns_to_ip() {
        let server_name = ServerName::try_from("127.0.0.1").unwrap();
        let address = Address::Dns(("www.google.com".to_string(), 80));
        assert!(!address.is_host_sni_equal(&server_name));
    }

    #[test]
    fn test_to_string_from_scheme_default_http_ip() {
        let address = Address::Socket("127.0.0.1:80".parse().unwrap());
        assert_eq!(address.to_string_from_scheme(Scheme::Http), "127.0.0.1")
    }

    #[test]
    fn test_to_string_from_scheme_default_https_ip() {
        let address = Address::Socket("127.0.0.1:443".parse().unwrap());
        assert_eq!(address.to_string_from_scheme(Scheme::Https), "127.0.0.1")
    }

    #[test]
    fn test_to_string_from_scheme_http_dns() {
        let address = Address::Dns(("www.google.com".to_string(), 80));
        assert_eq!(
            address.to_string_from_scheme(Scheme::Http),
            "www.google.com"
        )
    }

    #[test]
    fn test_to_string_from_scheme_https_dns() {
        let address = Address::Dns(("www.google.com".to_string(), 443));
        assert_eq!(
            address.to_string_from_scheme(Scheme::Https),
            "www.google.com"
        )
    }

    #[test]
    fn test_to_string_from_scheme_http_ip_port() {
        let address = Address::Socket("127.0.0.1:8080".parse().unwrap());
        assert_eq!(
            address.to_string_from_scheme(Scheme::Http),
            "127.0.0.1:8080"
        )
    }

    #[test]
    fn test_to_string_from_scheme_https_ip_port() {
        let address = Address::Socket("127.0.0.1:8080".parse().unwrap());
        assert_eq!(
            address.to_string_from_scheme(Scheme::Https),
            "127.0.0.1:8080"
        )
    }

    #[test]
    fn test_to_string_from_scheme_http_dns_port() {
        let address = Address::Dns(("www.google.com".to_string(), 8080));
        assert_eq!(
            address.to_string_from_scheme(Scheme::Http),
            "www.google.com:8080"
        )
    }

    #[test]
    fn test_to_string_from_scheme_https_dns_port() {
        let address = Address::Dns(("www.google.com".to_string(), 8080));
        assert_eq!(
            address.to_string_from_scheme(Scheme::Https),
            "www.google.com:8080"
        )
    }
}
