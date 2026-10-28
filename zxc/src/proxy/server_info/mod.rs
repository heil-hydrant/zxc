pub mod address;
pub mod json;
pub mod scheme;

use std::fmt::{Display, Formatter};

use address::Address;
use address::error::AddressError;
pub use json::ServerInfoJson;
use rustls_pki_types::ServerName;
use scheme::Scheme;

// struct to store server info
#[derive(Debug, Clone)]
pub struct ServerInfo {
    address: Address,
    scheme: Scheme,
    sni: Option<ServerName<'static>>,
}

impl ServerInfo {
    pub fn new(
        address: Address,
        tls: bool,
        server_name: Option<ServerName<'static>>,
    ) -> Self {
        Self {
            address,
            scheme: if tls {
                Scheme::Https
            } else {
                Scheme::Http
            },
            sni: server_name,
        }
    }

    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn is_tls(&self) -> bool {
        self.scheme == Scheme::Https
    }

    pub fn set_sni(&mut self, server_name: ServerName<'static>) {
        self.sni = Some(server_name);
    }

    pub fn sni(&self) -> &ServerName<'static> {
        self.sni.as_ref().unwrap()
    }

    // Returns true if host and sni are not equal
    pub fn should_add_sni(&self) -> bool {
        !self
            .address
            .is_host_sni_equal(self.sni())
    }

    pub fn address_to_string(&self) -> String {
        self.address
            .to_string_from_scheme(self.scheme)
    }
}

impl TryFrom<ServerInfoJson> for ServerInfo {
    type Error = AddressError;

    fn try_from(info: ServerInfoJson) -> Result<Self, Self::Error> {
        let scheme = if info.http.is_some() {
            Scheme::Http
        } else {
            Scheme::Https
        };
        let address = Address::try_from((info.host.as_str(), scheme))?;
        let server_name = if scheme == Scheme::Https {
            let sni = address
                .parse_sni(info.sni.as_deref())?
                .to_owned();
            Some(sni)
        } else {
            None
        };
        Ok(Self {
            address,
            scheme,
            sni: server_name,
        })
    }
}

impl Display for ServerInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}|{}", self.scheme, self.address)
    }
}

#[cfg(test)]
mod tests {
    use json::ServerInfoJson;
    use rustls_pki_types::ServerName;

    use super::*;

    #[test]
    fn test_server_info_from_http_host_only() {
        let json = ServerInfoJson::new(
            "www.google.com".to_string(),
            Scheme::Http,
            None,
        );
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Dns(("www.google.com".to_string(), 80));
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Http);
        assert!(!server_info.is_tls());
        assert!(server_info.sni.is_none());
    }

    #[test]
    fn test_server_info_from_https_host_only() {
        let json = ServerInfoJson::new(
            "www.google.com".to_string(),
            Scheme::Https,
            None,
        );
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Dns(("www.google.com".to_string(), 443));
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Https);
        assert!(server_info.is_tls());
        let server_name = ServerName::try_from("www.google.com").unwrap();
        assert_eq!(server_info.sni(), &server_name);
    }

    #[test]
    fn test_server_info_from_http_host_with_port() {
        let json = ServerInfoJson::new(
            "www.google.com:8080".to_string(),
            Scheme::Http,
            None,
        );
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Dns(("www.google.com".to_string(), 8080));
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Http);
        assert!(!server_info.is_tls());
        assert!(server_info.sni.is_none());
    }

    #[test]
    fn test_server_info_from_https_host_with_port() {
        let json = ServerInfoJson::new(
            "www.google.com:8080".to_string(),
            Scheme::Https,
            None,
        );
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Dns(("www.google.com".to_string(), 8080));
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Https);
        assert!(server_info.is_tls());
        let server_name = ServerName::try_from("www.google.com").unwrap();
        assert_eq!(server_info.sni(), &server_name);
    }

    #[test]
    fn test_server_info_from_http_ip() {
        let json =
            ServerInfoJson::new("127.0.0.1".to_string(), Scheme::Http, None);
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Socket("127.0.0.1:80".parse().unwrap());
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Http);
        assert!(!server_info.is_tls());
        assert!(server_info.sni.is_none());
    }

    #[test]
    fn test_server_info_from_http_ip_with_port() {
        let json = ServerInfoJson::new(
            "127.0.0.1:8080".to_string(),
            Scheme::Http,
            None,
        );
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Socket("127.0.0.1:8080".parse().unwrap());
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Http);
        assert!(!server_info.is_tls());
        assert!(server_info.sni.is_none());
    }

    #[test]
    fn test_server_info_from_https_ip() {
        let json =
            ServerInfoJson::new("127.0.0.1".to_string(), Scheme::Https, None);
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Socket("127.0.0.1:443".parse().unwrap());
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Https);
        assert!(server_info.is_tls());
        let server_name = ServerName::try_from("127.0.0.1").unwrap();
        assert_eq!(server_info.sni(), &server_name);
    }

    #[test]
    fn test_server_info_from_https_ip_with_port() {
        let json = ServerInfoJson::new(
            "127.0.0.1:8080".to_string(),
            Scheme::Https,
            None,
        );
        let server_info = ServerInfo::try_from(json).unwrap();
        let addr = Address::Socket("127.0.0.1:8080".parse().unwrap());
        assert_eq!(*server_info.address(), addr);
        assert_eq!(server_info.scheme(), Scheme::Https);
        assert!(server_info.is_tls());
        let server_name = ServerName::try_from("127.0.0.1").unwrap();
        assert_eq!(server_info.sni(), &server_name);
    }
}
