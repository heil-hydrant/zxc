#[path = "domain_list.rs"]
pub mod domain_list;
pub use domain_list::*;

// Enum which contains the included/exluded DomainList
#[cfg_attr(any(test, debug_assertions), derive(PartialEq))]
#[derive(Debug)]
pub enum DomainFilter {
    Include(DomainList),
    Exclude(DomainList),
}

impl DomainFilter {
    pub fn contains(&self, host: &String) -> bool {
        match self {
            DomainFilter::Include(list) => list.contains(host),
            DomainFilter::Exclude(list) => !list.contains(host),
        }
    }
}
