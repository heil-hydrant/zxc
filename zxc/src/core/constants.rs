pub const CAPACITY_2MB: usize = 65536 * 32;

pub const PROXY: &str = "proxy";
pub const COMMANDER: &str = "commander";
pub const INTERCEPTOR: &str = "interceptor";
pub const HISTORY: &str = "history";
pub const REPEATER: &str = "repeater";
pub const ADDONS: &str = "addons";

pub const HISTORY_STATE_FILE: &str = ".history.state";

// Extended attributes written to logged request files.
pub const XATTR_HOST: &str = "user.host";
pub const XATTR_HTTP: &str = "user.http";
pub const XATTR_SNI: &str = "user.sni";
pub const XATTR_IS_H2: &str = "user.is_h2";
pub const XATTR_AUTHORITY: &str = "user.authority";
pub const XATTR_H2_SCHEME: &str = "user.h2_scheme";

#[cfg(target_os = "linux")]
pub const REQUEST_XATTRS: [&str; 6] = [
    XATTR_HOST,
    XATTR_HTTP,
    XATTR_SNI,
    XATTR_IS_H2,
    XATTR_AUTHORITY,
    XATTR_H2_SCHEME,
];
