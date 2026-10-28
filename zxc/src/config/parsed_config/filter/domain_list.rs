use std::fmt::Debug;

use tracing::{error, trace};
use wildcard::Wildcard;

// Struct which contains domain names as list of string and wildcards
pub struct DomainList {
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
    /* Steps:
     *      1. Check wildcard list
     *      2. If no result, check string list
     */

    pub fn contains(&self, host: &String) -> bool {
        // 1. Check wildcard list
        if let Some(wild_list) = &self.wildcard
            && wild_list
                .iter()
                .any(|r| r.is_match(host.as_bytes()))
        {
            return true;
        }
        // 2. check string list
        if let Some(str_list) = &self.str
            && str_list.binary_search(host).is_ok()
        {
            return true;
        }
        false
    }
}

/* Steps:
 *      1. Separate wildcard list and string list
 *      2. Convert wildcard list to Wildcard
 */

impl From<Vec<String>> for DomainList {
    fn from(v: Vec<String>) -> Self {
        // 1. Separate string list and wildcard list
        let (wildcard, mut str): (Vec<_>, Vec<_>) = v
            .into_iter()
            .partition(|val| val.contains('*'));

        trace!("wildcard| {:?}", wildcard);

        // 2. Build wildcard list
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

        assert!(domain_list.contains(&"www.reddit.com".to_string()));
        assert!(domain_list.contains(&"chat.reddit.com".to_string()));
        assert!(domain_list.contains(&"www.google.com".to_string()));
        assert!(domain_list.contains(&"www.youtube.com".to_string()));
        assert!(!domain_list.contains(&"www.bing.com".to_string()));

        let wild = domain_list.wildcard.unwrap();
        assert!(wild[0].is_match(b"www.reddit.com"));
    }
}
