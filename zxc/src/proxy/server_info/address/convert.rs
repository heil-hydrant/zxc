use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use super::AddressError;
use crate::proxy::prelude::*;

/* Description:
 *      Method to split host and port from an &str.
 *      input can be:
 *          - host:port
 *          - host without port, in which case return 0.
 *
 * Steps:
 *      1. rplit_once(':') to get host and port
 *      2. If host is empty, return error
 *      3. Parse port to u16, if no port is found, set port to 0
 *
 * Error:
 *      AddressError::NoHost    [2]
 *      AddressError::PortParse [3]
 */

pub fn split_host_and_port(addr: &str) -> Result<(&str, u16), AddressError> {
    let (host, port_str) = addr
        .rsplit_once(':')
        .unwrap_or((addr, ""));

    if host.is_empty() {
        return Err(AddressError::NoHost(addr.to_string()));
    }

    let port = if !port_str.is_empty() {
        port_str.parse::<u16>()?
    } else {
        0
    };

    Ok((host, port))
}

impl TryFrom<&[u8]> for Address {
    type Error = AddressError;

    fn try_from(val: &[u8]) -> Result<Self, Self::Error> {
        let addr_str = std::str::from_utf8(val)?;
        Self::try_from(addr_str)
    }
}

impl TryFrom<&str> for Address {
    type Error = AddressError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        // 1. Try to convert &str to SocketAddr
        if let Ok(socket_addr) = SocketAddr::from_str(val) {
            return Ok(Address::Socket(socket_addr));
        }

        // 2. Check if ipaddress
        if let Ok(ip) = IpAddr::from_str(val) {
            return Ok(Address::Socket(SocketAddr::new(ip, 0)));
        }

        // Else, treat it as DNS (host, port)
        let (host, port) = split_host_and_port(val)?;
        Ok(Address::Dns((host.to_string(), port)))
    }
}

impl TryFrom<(&str, Scheme)> for Address {
    type Error = AddressError;

    fn try_from((host, scheme): (&str, Scheme)) -> Result<Self, Self::Error> {
        let mut address = Address::try_from(host)?;
        // If port is 0, set it to scheme default
        if address.port() == 0 {
            address.set_port(scheme.default_port());
        }
        Ok(address)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_split_with_port() {
        let (host, port) = split_host_and_port("www.google.com:8080").unwrap();
        assert_eq!(host, "www.google.com");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_split_without_port() {
        let (host, port) = split_host_and_port("www.google.com").unwrap();
        assert_eq!(host, "www.google.com");
        assert_eq!(port, 0);
    }

    #[test]
    fn test_split_with_empty_host() {
        let result = split_host_and_port(":8080");
        assert!(matches!(result, Err(AddressError::NoHost(_))));
    }

    #[test]
    fn test_split_large_port() {
        let result = split_host_and_port("www.google.com:65536");
        assert!(matches!(result, Err(AddressError::PortParse(_))));
    }

    #[test]
    fn test_address_from_empty_str() {
        let address = Address::try_from("");
        assert!(matches!(address, Err(AddressError::NoHost(_))));
    }

    #[test]
    fn test_address_from_str_ip_only() {
        let address = Address::try_from("127.0.0.1").unwrap();
        assert_eq!(
            address,
            Address::Socket(SocketAddr::from_str("127.0.0.1:0").unwrap())
        )
    }

    #[test]
    fn test_address_from_str_ip_with_port() {
        let address = Address::try_from("127.0.0.1:8080").unwrap();
        assert_eq!(
            address,
            Address::Socket(SocketAddr::from_str("127.0.0.1:8080").unwrap())
        )
    }

    #[test]
    fn test_address_from_str_dns_only() {
        let address = Address::try_from("www.google.com").unwrap();
        assert_eq!(address, Address::Dns(("www.google.com".to_string(), 0)))
    }

    #[test]
    fn test_address_from_str_dns_with_port() {
        let address = Address::try_from("www.google.com:8080").unwrap();
        assert_eq!(address, Address::Dns(("www.google.com".to_string(), 8080)))
    }

    #[test]
    fn test_address_from_str_scheme_http() {
        let address = Address::try_from(("127.0.0.1", Scheme::Http)).unwrap();
        assert_eq!(
            address,
            Address::Socket(SocketAddr::from_str("127.0.0.1:80").unwrap())
        )
    }

    #[test]
    fn test_address_from_str_scheme_https() {
        let address = Address::try_from(("127.0.0.1", Scheme::Https)).unwrap();
        assert_eq!(address, Address::Socket("127.0.0.1:443".parse().unwrap()))
    }

    #[test]
    fn test_address_from_str_scheme_http_no_port() {
        let address = Address::try_from(("127.0.0.1", Scheme::Http)).unwrap();
        assert_eq!(address, Address::Socket("127.0.0.1:80".parse().unwrap()))
    }

    #[test]
    fn test_address_from_str_scheme_https_no_port() {
        let address = Address::try_from(("127.0.0.1", Scheme::Https)).unwrap();
        assert_eq!(address, Address::Socket("127.0.0.1:443".parse().unwrap()))
    }

    #[test]
    fn test_address_from_str_scheme_http_with_port() {
        let address =
            Address::try_from(("127.0.0.1:8080", Scheme::Http)).unwrap();
        assert_eq!(address, Address::Socket("127.0.0.1:8080".parse().unwrap()))
    }

    #[test]
    fn test_address_from_str_scheme_https_with_port() {
        let address =
            Address::try_from(("127.0.0.1:8443", Scheme::Https)).unwrap();
        assert_eq!(address, Address::Socket("127.0.0.1:8443".parse().unwrap()))
    }
}
