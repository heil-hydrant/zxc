use tracing::error;
use wildcard::Wildcard;
mod from_vec_string;
use std::fmt::Debug;

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
