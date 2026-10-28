use std::ops::Add;

use clap::Args;
use clap::builder::NonEmptyStringValueParser;
use serde::{Deserialize, Serialize};

use crate::config::*;

// Struct for command line arguments + Local Config
#[cfg_attr(test, derive(PartialEq, Clone, Default))]
#[derive(Args, Debug, Serialize, Deserialize)]
pub struct ProxyArgs {
    /// Proxy port to use
    #[arg(short, long = "port")]
    pub port: Option<u16>,
    /// List of domains to intercept
    #[arg(
        short,
        long = "include",
        conflicts_with = "excluded_domains",
        value_delimiter = ',',
        value_parser = NonEmptyStringValueParser::new()
    )]
    pub included_domains: Option<Vec<String>>,
    /// List of domains to not intercept
    #[arg(short,
        long = "exclude",
        conflicts_with = "included_domains",
        value_delimiter = ',',
        value_parser = NonEmptyStringValueParser::new()
    )]
    pub excluded_domains: Option<Vec<String>>,
    /// Relay ws connections
    #[arg(long = "no-ws", action = clap::ArgAction::SetTrue)]
    pub no_ws: Option<bool>,
}

impl ProxyArgs {
    /* Steps:
     *      1. If port is 8080, remove it
     *      2. Remove empty and duplicate values from included_domains and
     *         excluded_domains
     *      3. If all fields are empty and no_ws is false, return None
     */

    pub fn sanitize(mut self) -> Option<ProxyArgs> {
        // if port is 8080 , remove it
        if self.port == Some(8080) {
            self.port = None;
        }
        if self.no_ws == Some(false) {
            self.no_ws.take();
        }
        sanitize_option_vec_string(&mut self.included_domains);
        sanitize_option_vec_string(&mut self.excluded_domains);
        if self.port.is_some()
            || self.included_domains.is_some()
            || self.excluded_domains.is_some()
            || self.no_ws.is_some()
        {
            Some(self)
        } else {
            None
        }
    }
}

// new + old
impl Add for ProxyArgs {
    type Output = ProxyArgs;

    fn add(self, rhs: Self) -> Self::Output {
        let port = self.port.or(rhs.port);
        let no_ws = match (self.no_ws, rhs.no_ws) {
            (Some(a), Some(b)) => Some(a || b),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        // if both include and exclude are present include is given prefrence
        let included_domains: Option<Vec<String>> =
            if self.included_domains.is_some()
                || rhs.included_domains.is_some()
            {
                add_option_vec(self.included_domains, rhs.included_domains)
            } else {
                None
            };

        let excluded_domains = if included_domains.is_none()
            || self.excluded_domains.is_some()
            || rhs.excluded_domains.is_some()
        {
            add_option_vec(self.excluded_domains, rhs.excluded_domains)
        } else {
            None
        };

        ProxyArgs {
            port,
            included_domains,
            excluded_domains,
            no_ws,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_proxyargs_sanitize_none() {
        let proxy = ProxyArgs::default();
        assert!(proxy.sanitize().is_none());
    }

    #[test]
    fn test_proxyargs_sanitize_no_ws() {
        let proxy = ProxyArgs {
            no_ws: Some(true),
            ..Default::default()
        };
        assert!(proxy.sanitize().is_some());
    }

    #[test]
    fn test_proxyargs_sanitize_included_domains_only() {
        let proxy = ProxyArgs {
            included_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        assert!(proxy.sanitize().is_some());
    }

    #[test]
    fn test_proxyargs_sanitize_excluded_domains_only() {
        let proxy = ProxyArgs {
            excluded_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        assert!(proxy.sanitize().is_some());
    }

    #[test]
    fn test_proxyargs_sanitize_port_8080() {
        let proxy = ProxyArgs {
            port: Some(8080),
            ..Default::default()
        };
        assert!(proxy.sanitize().is_none());
    }

    #[test]
    fn test_proxyargs_sanitize_port_8081() {
        let proxy = ProxyArgs {
            port: Some(8081),
            no_ws: Some(false),
            ..Default::default()
        };
        if let Some(proxy) = proxy.sanitize() {
            assert!(proxy.no_ws.is_none());
        } else {
            panic!()
        }
    }

    // ---- Add

    // Port
    #[test]
    fn test_proxyargs_add_new_port_old_no_port() {
        let new = ProxyArgs {
            port: Some(8081),
            ..Default::default()
        };
        let old = ProxyArgs::default();
        assert_eq!(new.clone() + old, new);
    }

    #[test]
    fn test_proxyargs_add_new_no_port_old_port() {
        let new = ProxyArgs::default();
        let old = ProxyArgs {
            port: Some(8081),
            ..Default::default()
        };
        assert_eq!(new + old.clone(), old);
    }

    // Include domains
    #[test]
    fn test_proxyargs_add_new_included_domains_old_no_included_domains() {
        let new = ProxyArgs {
            included_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        let old = ProxyArgs::default();
        assert_eq!(new.clone() + old, new);
    }

    #[test]
    fn test_proxyargs_add_new_no_included_domains_old_included_domains() {
        let new = ProxyArgs::default();
        let old = ProxyArgs {
            included_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        assert_eq!(new + old.clone(), old);
    }

    #[test]
    fn test_proxyargs_add_new_included_domains_old_included_domains() {
        let new = ProxyArgs {
            included_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        let old = ProxyArgs {
            included_domains: Some(vec!["b".to_string()]),
            ..Default::default()
        };
        let verify = ProxyArgs {
            included_domains: Some(vec!["a".to_string(), "b".to_string()]),
            ..Default::default()
        };
        assert_eq!(new + old.clone(), verify);
    }

    // exclude domains
    #[test]
    fn test_proxyargs_add_new_excluded_domains_old_no_excluded_domains() {
        let new = ProxyArgs {
            excluded_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        let old = ProxyArgs::default();
        assert_eq!(new.clone() + old, new);
    }

    #[test]
    fn test_proxyargs_add_new_no_excluded_domains_old_excluded_domains() {
        let new = ProxyArgs::default();
        let old = ProxyArgs {
            excluded_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        assert_eq!(new + old.clone(), old);
    }

    #[test]
    fn test_proxyargs_add_new_excluded_domains_old_excluded_domains() {
        let new = ProxyArgs {
            excluded_domains: Some(vec!["a".to_string()]),
            ..Default::default()
        };
        let old = ProxyArgs {
            excluded_domains: Some(vec!["b".to_string()]),
            ..Default::default()
        };
        let verify = ProxyArgs {
            excluded_domains: Some(vec!["a".to_string(), "b".to_string()]),
            ..Default::default()
        };
        assert_eq!(new + old.clone(), verify);
    }
}
