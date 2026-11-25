use std::fmt::Debug;

use tracing::{error, trace};
use wildcard::Wildcard;

// Enum which contains the included/exluded DomainList
#[cfg_attr(any(test, debug_assertions), derive(PartialEq))]
#[derive(Debug)]
pub(super) enum DomainFilter {
    Include(DomainList),
    Exclude(DomainList),
}

impl DomainFilter {
    pub(super) fn contains(&self, host: &str) -> bool {
        match self {
            DomainFilter::Include(list) => list.contains(host),
            DomainFilter::Exclude(list) => !list.contains(host),
        }
    }
}

// Struct which contains domain names as list of string and wildcards
pub(super) struct DomainList {
    str: Option<Vec<String>>,
    wildcard: Option<Vec<Wildcard<'static>>>,
}

impl Debug for DomainList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainList")
            .field("str", &self.str)
            //.field("wildcard", &self.wildcard)
            .finish()
    }
}

#[cfg(any(test, debug_assertions))]
impl PartialEq for DomainList {
    fn eq(&self, other: &Self) -> bool {
        self.str == other.str
    }
}

impl DomainList {
    pub(super) fn contains(&self, host: &str) -> bool {
        let host_only = host_without_port(host);
        self.matches(host) || (host_only != host && self.matches(host_only))
    }

    fn matches(&self, host: &str) -> bool {
        if let Some(wild_list) = &self.wildcard
            && wild_list
                .iter()
                .any(|r| r.is_match(host.as_bytes()))
        {
            return true;
        }
        if let Some(str_list) = &self.str
            && str_list
                .binary_search_by(|s| s.as_str().cmp(host))
                .is_ok()
        {
            return true;
        }
        false
    }
}

// `Address` renders hosts as `host:port` or `[ipv6]:port`. Entries
// without a port match the host part, while entries with a port keep
// matching the full `host:port`.
fn host_without_port(host: &str) -> &str {
    match host.rsplit_once(':') {
        Some((head, port))
            if port.parse::<u16>().is_ok()
                && (head.starts_with('[') || !head.contains(':')) =>
        {
            head
        }
        _ => host,
    }
}

impl From<Vec<String>> for DomainList {
    fn from(v: Vec<String>) -> Self {
        // Separate string list and wildcard list
        let (wildcard, mut str): (Vec<_>, Vec<_>) = v
            .into_iter()
            .partition(|val| val.contains('*'));

        trace!("wildcard| {:?}", wildcard);

        let wildcard = wildcard
            .into_iter()
            .filter_map(|val| match Wildcard::from_owned(val.into()) {
                Ok(r) => Some(r),
                Err(e) => {
                    error!("Error converting to wildcard| {}", e);
                    None
                }
            })
            .collect::<Vec<Wildcard<'static>>>();

        DomainList {
            str: if str.is_empty() {
                None
            } else {
                str.sort_unstable();
                Some(str)
            },
            wildcard: if wildcard.is_empty() {
                None
            } else {
                Some(wildcard)
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_vec_to_domain_list() {
        let vec = vec![
            "*.reddit.com".to_string(),
            "www.google.com".to_string(),
            "www.youtube.com".to_string(),
        ];
        let domain_list = DomainList::from(vec);
        assert_eq!(
            domain_list.str,
            Some(vec![
                "www.google.com".to_string(),
                "www.youtube.com".to_string()
            ])
        );

        //assert_eq!(
        //    domain_list.wildcard,
        //    Some(vec![Wildcard::from_owned("*.reddit.com").unwrap()])
        //);

        assert!(domain_list.contains("www.reddit.com"));
        assert!(domain_list.contains("chat.reddit.com"));
        assert!(domain_list.contains("www.google.com"));
        assert!(domain_list.contains("www.youtube.com"));
        assert!(!domain_list.contains("www.bing.com"));

        let wild = domain_list.wildcard.unwrap();
        assert!(wild[0].is_match(b"www.reddit.com"));
    }

    #[test]
    fn test_contains_host_with_port() {
        let domain_list = DomainList::from(vec![
            "www.google.com".to_string(),
            "*.example.com".to_string(),
        ]);

        assert!(domain_list.contains("www.google.com:443"));
        assert!(domain_list.contains("www.google.com:8080"));
        assert!(domain_list.contains("foo.example.com:443"));
    }

    #[test]
    fn test_contains_port_specific_entry() {
        let domain_list = DomainList::from(vec![
            "www.facebook.com:80".to_string(),
            "*.google.com:443".to_string(),
        ]);

        assert!(domain_list.contains("www.facebook.com:80"));
        assert!(!domain_list.contains("www.facebook.com:443"));
        assert!(domain_list.contains("www.google.com:443"));
        assert!(!domain_list.contains("www.google.com:80"));
    }

    #[test]
    fn test_contains_ipv4_host_with_port() {
        let domain_list = DomainList::from(vec!["127.0.0.1".to_string()]);

        assert!(domain_list.contains("127.0.0.1:8080"));
    }

    #[test]
    fn test_contains_bracketed_ipv6_host_with_port() {
        let domain_list = DomainList::from(vec!["[::1]".to_string()]);

        assert!(domain_list.contains("[::1]:443"));
    }

    #[test]
    fn test_exclude_domain_with_port() {
        let filter = DomainFilter::Exclude(DomainList::from(vec![
            "*.example.com".to_string(),
        ]));

        assert!(!filter.contains("foo.example.com:443"));
        assert!(filter.contains("foo.other.com:443"));
    }
}
