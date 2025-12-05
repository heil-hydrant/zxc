mod filter;
use filter::DomainFilter;
use filter::domain_list::DomainList;
use mime::ContentType;
use mime::from_extension::EXTENSION_MAP;
use tracing::trace;

use super::GlobalConfig;
use super::local::proxy::ProxyArgs;
use crate::config::misc::{add_option_vec, sanitize_option_vec_string};

// Configuration Struct which holds the config informartion
#[cfg_attr(any(test, debug_assertions), derive(PartialEq))]
#[derive(Debug)]
pub struct Config {
    filter_domains: Option<DomainFilter>,
    excluded_content_types: Option<Vec<ContentType>>,
    excluded_extensions: Option<Vec<String>>,
    with_ws: bool,
}

impl Config {
    /* Steps:
     *      1. Get Excluded Content Types and Extensions from global config
     *
     *      2. Get with_ws from global config and no_ws from local config and
     *         (global || !local) to get with_ws
     *
     *            g l f
     *            -----
     *            0 0 0
     *            1 0 1
     *            0 1 0
     *            1 1 0
     *
     *      3. Get filter_domains from local config and global config
     *      by calling combine_filter
     *
     *      4. Sanitize, if any field is Some and with_ws is false
     *      return Some(Config) else return None
     */

    pub fn build(
        local_config: Option<ProxyArgs>,
        mut global_config: Option<GlobalConfig>,
    ) -> Option<Config> {
        // 1. Exclude content types and extensions
        let excluded_content_types = global_config
            .as_mut()
            .and_then(|gc| gc.excluded_content_types.take());

        let excluded_extensions = global_config
            .as_mut()
            .and_then(|gc| gc.excluded_extensions.take());

        let global_ws = global_config
            .as_ref()
            .is_some_and(|gc| gc.with_ws.unwrap_or(false));

        let local_ws = local_config
            .as_ref()
            .is_some_and(|lc| lc.no_ws.unwrap_or(false));

        let with_ws = global_ws && !local_ws;

        // 5. Combine local and global config
        let filter_domains = Self::combine_filter(local_config, global_config);

        // sanitize
        if !with_ws
            || filter_domains.is_some()
            || excluded_content_types.is_some()
            || excluded_extensions.is_some()
        {
            Some(Config {
                filter_domains,
                excluded_content_types,
                excluded_extensions,
                with_ws,
            })
        } else {
            None
        }
    }

    /* Description:
     *      Combine Local and Global config to get DomainFilter
     *
     * Steps:
     *      1. If no config, return None
     *
     *      2. If only global config, return DomainList::Exclude from
     *         global_config.excluded_domains since, global_config only
     *         contains excluded_domains
     *
     *      3. If only local config, either include or exclude is present
     *
     *          a. if local_config.included_domains is Some, return
     *          DomainList::Include from local_config.included_domains
     *
     *          b. else return DomainList::Exclude from
     *          local_config.excluded_domains
     *
     *      4. If both config present,
     *          a. if local_config.included_domains is Some, return
     *          DomainList::Include from local_config.included_domains
     *
     *          b. else
     *              1. combine local_config.excluded_domains and
     *                 global.excluded_domains by calling add_option_vec::add()
     *
     *              2. Sanitize the result by calling
     *                 sanitize_option_vec_string()
     *
     *              3. If list is some return DomainList::Exclude
     */

    fn combine_filter(
        local_config: Option<ProxyArgs>,
        global_config: Option<GlobalConfig>,
    ) -> Option<DomainFilter> {
        match (local_config, global_config) {
            // 1. No config
            (None, None) => None,

            // 2. Only global config
            (None, Some(gc)) => {
                trace!("no local config");
                gc.excluded_domains
                    .map(|list| DomainFilter::Exclude(DomainList::from(list)))
            }

            // 3. Only local config
            (Some(lc), None) => {
                trace!("no global config");
                if let Some(list) = lc.included_domains {
                    Some(DomainFilter::Include(DomainList::from(list)))
                } else {
                    lc.excluded_domains.map(|list| {
                        DomainFilter::Exclude(DomainList::from(list))
                    })
                }
            }

            // 4. Both config
            (Some(lc), Some(gc)) => {
                trace!("local and global config");
                if let Some(llist) = lc.included_domains {
                    Some(DomainFilter::Include(DomainList::from(llist)))
                }
                // 4.b. local-exclude + global-exclude
                else {
                    let mut list = add_option_vec(
                        gc.excluded_domains,
                        lc.excluded_domains,
                    );
                    sanitize_option_vec_string(&mut list);

                    list.map(|list| {
                        DomainFilter::Exclude(DomainList::from(list))
                    })
                }
            }
        }
    }

    pub fn should_proxy(&self, host: &String) -> bool {
        self.filter_domains
            .as_ref()
            .is_none_or(|filter| filter.contains(host))
    }

    /* Description:
     *      Checks if given extension should be logged.
     *
     * Steps:
     *      1. If extension is not empty, make it lowercase
     *      2. Check if extension is not in excluded_extensions,
     *      3. If false, then get ContentType from extension by calling
     *         content_type_from_extension() and check if ContentType is in
     *         excluded_content_types
     */

    pub fn should_log(&self, mut ext: String) -> bool {
        if !ext.is_empty() {
            ext.make_ascii_lowercase();
            if self.in_excluded_extensions(&ext) {
                return false;
            } else if let Some(ct) = EXTENSION_MAP.get(ext.as_str()).copied() {
                return !self.in_excluded_content_types(ct);
            }
        }
        true
    }

    pub fn in_excluded_content_types(&self, ct: ContentType) -> bool {
        if self
            .excluded_content_types
            .as_ref()
            .is_some_and(|ect| ect.contains(&ct))
        {
            trace!("in excluded content types| {}", ct);
            return true;
        }
        false
    }

    pub fn in_excluded_extensions(&self, extension: &String) -> bool {
        if self
            .excluded_extensions
            .as_ref()
            .is_some_and(|ext| ext.binary_search(extension).is_ok())
        {
            trace!("in excluded extensions| {}", extension);
            return true;
        }
        false
    }

    pub fn with_ws(&self) -> bool {
        self.with_ws
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_config_only_global() {
        let global_config = Some(GlobalConfig {
            excluded_domains: Some(vec!["*.google.com".to_string()]),
            excluded_content_types: Some(vec![
                ContentType::Image,
                ContentType::Video,
                ContentType::Audio,
            ]),
            excluded_extensions: Some(vec![
                "jpg".to_string(),
                "png".to_string(),
            ]),
            with_ws: None,
            addons: None,
        });

        let config = Config::build(None, global_config);
        let verify = Config {
            filter_domains: Some(DomainFilter::Exclude(DomainList::from(
                vec!["*.google.com".to_string()],
            ))),
            excluded_content_types: Some(vec![
                ContentType::Image,
                ContentType::Video,
                ContentType::Audio,
            ]),
            excluded_extensions: Some(vec![
                "jpg".to_string(),
                "png".to_string(),
            ]),
            with_ws: false,
        };

        assert_eq!(config.unwrap(), verify);
    }

    #[test]
    fn test_build_config_only_local() {
        let local_config = Some(ProxyArgs {
            port: None,
            included_domains: Some(vec!["*.google.com".to_string()]),
            excluded_domains: None,
            no_ws: None,
        });

        let config = Config::build(local_config, None);
        let verify = Config {
            filter_domains: Some(DomainFilter::Include(DomainList::from(
                vec!["*.google.com".to_string()],
            ))),
            excluded_content_types: None,
            excluded_extensions: None,
            with_ws: false,
        };

        assert_eq!(config.unwrap(), verify);
    }

    #[test]
    fn test_parse_config_local_no_global_no() {
        let local_config: Option<ProxyArgs> = None;
        let global_config: Option<GlobalConfig> = None;
        let filter = Config::combine_filter(local_config, global_config);
        assert_eq!(filter, None);
    }

    #[test]
    fn test_parse_config_local_no_global_yes() {
        let elist = vec!["reddit.com".to_string(), "*.google.com".to_string()];
        let local_config: Option<ProxyArgs> = None;
        let global_config: Option<GlobalConfig> = Some(GlobalConfig {
            excluded_domains: Some(elist.clone()),
            excluded_content_types: None,
            excluded_extensions: None,
            with_ws: None,
            addons: None,
        });
        let filter = Config::combine_filter(local_config, global_config);
        assert_eq!(
            filter,
            Some(DomainFilter::Exclude(DomainList::from(elist)))
        );
    }

    #[test]
    fn test_parse_config_local_exclude_global_no() {
        let elist = vec!["reddit.com".to_string(), "*.google.com".to_string()];
        let local_config: Option<ProxyArgs> = Some(ProxyArgs {
            included_domains: None,
            excluded_domains: Some(elist.clone()),
            port: Some(8080),
            no_ws: None,
        });
        let global_config: Option<GlobalConfig> = None;
        let filter = Config::combine_filter(local_config, global_config);
        assert_eq!(
            filter,
            Some(DomainFilter::Exclude(DomainList::from(elist)))
        );
    }

    #[test]
    fn test_parse_config_local_include_global_no() {
        let elist = vec!["reddit.com".to_string(), "*.google.com".to_string()];
        let local_config: Option<ProxyArgs> = Some(ProxyArgs {
            included_domains: Some(elist.clone()),
            excluded_domains: None,
            port: Some(8080),
            no_ws: None,
        });
        let global_config: Option<GlobalConfig> = None;
        let filter = Config::combine_filter(local_config, global_config);
        assert_eq!(
            filter,
            Some(DomainFilter::Include(DomainList::from(elist)))
        )
    }

    #[test]
    fn test_parse_config_local_include_exclude_global_no() {
        let elist = vec!["reddit.com".to_string(), "*.google.com".to_string()];
        let ilist = vec!["*.youtube.com".to_string()];
        let local_config: Option<ProxyArgs> = Some(ProxyArgs {
            included_domains: Some(ilist.clone()),
            excluded_domains: Some(elist.clone()),
            port: Some(8080),
            no_ws: None,
        });
        let global_config: Option<GlobalConfig> = None;
        let filter = Config::combine_filter(local_config, global_config);
        assert_eq!(
            filter,
            Some(DomainFilter::Include(DomainList::from(ilist)))
        );
    }

    #[test]
    fn test_parse_config_local_include_exclude_global_yes() {
        let elist = vec!["reddit.com".to_string(), "*.google.com".to_string()];
        let ilist = vec!["*.youtube.com".to_string()];
        let local_config: Option<ProxyArgs> = Some(ProxyArgs {
            included_domains: Some(ilist.clone()),
            excluded_domains: Some(elist.clone()),
            port: Some(8080),
            no_ws: None,
        });
        let global_config: Option<GlobalConfig> = Some(GlobalConfig {
            excluded_domains: Some(elist.clone()),
            excluded_content_types: None,
            excluded_extensions: None,
            with_ws: None,
            addons: None,
        });
        let filter = Config::combine_filter(local_config, global_config);
        assert_eq!(
            filter,
            Some(DomainFilter::Include(DomainList::from(ilist)))
        );
    }

    #[test]
    fn test_parse_config_local_execlude_global_exclude() {
        let elist = vec!["reddit.com".to_string(), "*.google.com".to_string()];
        let local_config: Option<ProxyArgs> = Some(ProxyArgs {
            included_domains: None,
            excluded_domains: Some(elist.clone()),
            port: Some(8080),
            no_ws: None,
        });
        let gelist =
            vec!["*.youtube.com".to_string(), "reddit.com".to_string()];
        let global_config: Option<GlobalConfig> = Some(GlobalConfig {
            excluded_domains: Some(gelist.clone()),
            excluded_content_types: None,
            excluded_extensions: None,
            with_ws: None,
            addons: None,
        });
        let filter = Config::combine_filter(local_config, global_config);
        assert_eq!(
            filter,
            Some(DomainFilter::Exclude(DomainList::from(elist)))
        );
        if let Some(filter) = filter {
            assert!(filter.contains(&"www.reddit.com".to_string()));
            assert!(!filter.contains(&"www.youtube.com".to_string()));
            assert!(!filter.contains(&"www.google.com".to_string()));
        }
    }

    #[test]
    fn test_should_log() {
        let excluded_content_types =
            vec![ContentType::Image, ContentType::Video];
        let excluded_extensions = vec!["css".to_string(), "js".to_string()];
        let config: Config = Config {
            filter_domains: None,
            excluded_content_types: Some(excluded_content_types),
            excluded_extensions: Some(excluded_extensions),
            with_ws: false,
        };

        assert!(config.should_log("html".to_string()));
        assert!(!config.should_log("png".to_string()));
        assert!(!config.should_log("css".to_string()));
        assert!(!config.should_log("jpg".to_string()));
        assert!(!config.should_log("svg".to_string()));
        assert!(!config.should_log("mp4".to_string()));
    }
}
