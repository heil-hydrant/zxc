use std::fmt::Display;
pub mod from_extension;

use serde::{Deserialize, Serialize};

// Enum to represent different content types
#[derive(
    Debug,
    Default,
    PartialEq,
    Clone,
    Copy,
    Eq,
    Serialize,
    Deserialize,
    PartialOrd,
    Ord,
)]
pub enum ContentType {
    #[serde(rename = "app")]
    Application,
    #[serde(rename = "audio")]
    Audio,
    #[serde(rename = "font")]
    Font,
    #[serde(rename = "img")]
    Image,
    #[serde(rename = "msg")]
    Message,
    #[serde(rename = "model")]
    Model,
    #[serde(rename = "multipart")]
    Multipart,
    #[serde(rename = "txt")]
    Text,
    #[default]
    #[serde(rename = "ukn")]
    Unknown,
    #[serde(rename = "video")]
    Video,
}

impl ContentType {
    /* Description:
     *      Gets the content type from an accept header
     *
     * Steps:
     *      1. Split the accept header by "," and get the single entry.
     *      2. Split the single entry by "/" and get the main type.
     *      3. Get the content type from the main type by From<&str> trait.
     *      4. If current_ct is None, set current_ct to the content type.
     *      5. If the current type is different from the first, return None
     */

    pub fn from_accept_header(value: &str) -> Option<ContentType> {
        let mut current_ct: Option<ContentType> = None;
        for svalue in value.split(',') {
            if let Some(main_type) = svalue.trim().split('/').next() {
                let content_type: ContentType = ContentType::from(main_type);
                if current_ct.is_none() {
                    current_ct = Some(content_type);
                } else {
                    // If the current type is different from the first, return None
                    if current_ct != Some(content_type) {
                        return None;
                    }
                }
            }
        }

        if let Some(ct) = current_ct
            && matches!(ct, ContentType::Unknown)
        {
            return None;
        }
        current_ct
    }
}

impl Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&str> for ContentType {
    fn from(s: &str) -> Self {
        match s {
            "application" => ContentType::Application,
            "audio" => ContentType::Audio,
            "font" => ContentType::Font,
            "image" => ContentType::Image,
            "message" => ContentType::Message,
            "model" => ContentType::Model,
            "multipart" => ContentType::Multipart,
            "text" => ContentType::Text,
            "video" => ContentType::Video,
            _ => ContentType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_content_type() {
        let header_value = "text/html";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, Some(ContentType::Text));
    }

    #[test]
    fn test_same_content_types() {
        let header_value = "text/html, text/plain, text/css";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, Some(ContentType::Text));
    }

    #[test]
    fn test_mixed_content_types() {
        let header_value = "text/html, application/xhtml+xml, image/webp";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_content_types_with_quality() {
        let header_value = "application/xml;q=0.9, application/json;q=0.8";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, Some(ContentType::Application));
    }

    #[test]
    fn test_mixed_content_types_with_quality() {
        let header_value = "application/xml;q=0.9, image/webp;q=0.8";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_wildcard_type() {
        let header_value = "*/html, */xml;q=0.9";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_empty_header() {
        let header_value = "";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_invalid_format() {
        let header_value = "text, invalid/type";
        let result = ContentType::from_accept_header(header_value);
        assert_eq!(result, None);
    }
}
