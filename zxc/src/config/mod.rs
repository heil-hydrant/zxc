pub mod error;
pub mod global;
pub mod local;
mod misc;
mod parsed_config;
mod windows;

pub use error::ConfigError;
pub use global::GlobalConfig;
pub use global::addons::Addon;
pub use global::parser::{global_config_path, parse_global_config};
pub use local::CliArgs;
pub use local::io::{parse_local_config, write_local_config};
pub use local::proxy::ProxyArgs;
pub use misc::{add_option_vec, sanitize_option_vec_string};
pub use parsed_config::Config;
pub use windows::*;

pub const CONFIG_FILE_NAME: &str = "config.toml";
