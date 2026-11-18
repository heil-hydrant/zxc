use serde::{Deserialize, Serialize};

#[cfg_attr(any(test, debug_assertions), derive(PartialEq))]
#[derive(Deserialize, Serialize, Debug)]
pub struct Addon {
    pub prefix: String,
    pub request_flag: String,
    pub http_flag: Option<String>,
    pub https_flag: Option<String>,
    pub http2_flag: Option<String>,
    pub add_flag: Option<String>,
}

// Format: addon_name request_flag $file http/https_flag http2_flag add_flag
impl Addon {
    pub fn build_cmd(
        &self,
        name: &str,
        file: &str,
        tls: bool,
        is_h2: bool,
    ) -> String {
        let mut cmd = format!("{} {} {}", name, self.request_flag, file);
        let scheme_flag = if tls {
            self.https_flag.as_ref()
        } else {
            self.http_flag.as_ref()
        };
        if let Some(scheme_flag) = scheme_flag {
            cmd.push(' ');
            cmd.push_str(scheme_flag);
        }
        if is_h2 && let Some(http2_flag) = self.http2_flag.as_ref() {
            cmd.push(' ');
            cmd.push_str(http2_flag);
        }
        if let Some(add_flag) = self.add_flag.as_ref() {
            cmd.push(' ');
            cmd.push_str(add_flag);
        }
        cmd
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn build_addon(
        prefix: &str,
        request_flag: &str,
        http_flag: Option<&str>,
        https_flag: Option<&str>,
        http2_flag: Option<&str>,
        add_flag: Option<&str>,
    ) -> Addon {
        Addon {
            prefix: prefix.to_string(),
            request_flag: request_flag.to_string(),
            http_flag: http_flag.map(|s| s.to_string()),
            https_flag: https_flag.map(|s| s.to_string()),
            http2_flag: http2_flag.map(|s| s.to_string()),
            add_flag: add_flag.map(|s| s.to_string()),
        }
    }

    pub fn build_ffuf() -> Addon {
        build_addon(
            "z-",
            "-request",
            Some("-request-proto http"),
            None,
            Some("-http2"),
            Some("-w"),
        )
    }

    pub fn build_sqlmap() -> Addon {
        build_addon(
            "q-",
            "-r",
            None,
            Some("--force-ssl"),
            Some("--http2"),
            Some("--batch --dbs"),
        )
    }

    #[test]
    fn test_addon_build_cmd_ffuf() {
        let addon = build_ffuf();
        let cmd = addon.build_cmd("ffuf", "file", false, false);
        assert_eq!(cmd, "ffuf -request file -request-proto http -w");

        let cmd = addon.build_cmd("ffuf", "file", true, false);
        assert_eq!(cmd, "ffuf -request file -w");

        let cmd = addon.build_cmd("ffuf", "file", false, true);
        assert_eq!(
            cmd,
            "ffuf -request file -request-proto http -http2 -w"
        );

        let cmd = addon.build_cmd("ffuf", "file", true, true);
        assert_eq!(cmd, "ffuf -request file -http2 -w");
    }

    #[test]
    fn test_addon_build_cmd_sqlmap() {
        let addon = build_sqlmap();
        let cmd = addon.build_cmd("sqlmap", "file", true, false);
        assert_eq!(cmd, "sqlmap -r file --force-ssl --batch --dbs");

        let cmd = addon.build_cmd("sqlmap", "file", false, false);
        assert_eq!(cmd, "sqlmap -r file --batch --dbs");

        let cmd = addon.build_cmd("sqlmap", "file", true, true);
        assert_eq!(
            cmd,
            "sqlmap -r file --force-ssl --http2 --batch --dbs"
        );

        let cmd = addon.build_cmd("sqlmap", "file", false, true);
        assert_eq!(cmd, "sqlmap -r file --http2 --batch --dbs");
    }

    #[test]
    fn test_addon_build_cmd_h2_without_flag() {
        let addon = build_addon(
            "x-",
            "-r",
            None,
            None,
            None,
            Some("--batch"),
        );
        let cmd = addon.build_cmd("tool", "file", false, true);
        assert_eq!(cmd, "tool -r file --batch");
    }
}
