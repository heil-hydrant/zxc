use mime::ContentType;

use super::BodyHeader;
use crate::const_headers::*;
use crate::enums::content_encoding::ContentEncoding;
use crate::enums::transfer_types::{
    TransferType, cl_to_transfer_type, parse_and_remove_chunked
};
use crate::one_one::header_map::HeaderMap;

/* Steps:
 *      1. Create default BodyHeader.
 *
 *      2. Iterate through headers.
 *
 *      3. If header.key is "cl" or "Content-Length", and if
 *         body_headers.transfer_type is not set then convert content length to
 *         TransferType by calling cl_to_transfer_type()
 *
 *      4. If header.key is "te" or "Transfer-Encoding",
 *          a. build Vec<TransferEncoding> by calling match_compression() with
 *          header.value_as_str().
 *
 *          b. If chunked value is present, remove it and set transfer_type to
 *          TansferType of chunked
 *
 *      5. If header.key is "ce" or "Content-Encoding", set
 *         body_header.content_encoding to vec built by calling
 *         match_compression() with header.value_as_str().
 *
 *      6. If header.key is "ct" or "Content-Type", split at "/" to get
 *         main content type. Use From<&str> for ContentType to create
 *         ContentType from string. Assign to body_header.content_type.
 *
 *      7. If TransferType is Unknown, and if content_encoding or
 *         transfer_encoding or content_type is present, then set TransferType
 *         to Close
 *
 *      8. Call sanitize() on BodyHeader to remove empty values.
 *
 * Returns:
 *      Option<BodyHeader>
 */

impl From<&HeaderMap> for Option<BodyHeader> {
    fn from(header_map: &HeaderMap) -> Option<BodyHeader> {
        let mut body_headers = BodyHeader::default();
        header_map
            .headers()
            .iter()
            .for_each(|header| {
                let key = header.key_as_str();
                // 3. Content-Length
                if (key.eq_ignore_ascii_case(CL)
                    || key.eq_ignore_ascii_case(CONTENT_LENGTH))
                    && body_headers.transfer_type.is_none()
                {
                    let transfer_type =
                        cl_to_transfer_type(header.value_as_str());
                    body_headers.transfer_type = Some(transfer_type);
                }
                // 4.Transfer-Encoding
                if key.eq_ignore_ascii_case(TE)
                    || key.eq_ignore_ascii_case(TRANSFER_ENCODING)
                {
                    body_headers.transfer_encoding =
                        match_compression(header.value_as_str());

                    body_headers.transfer_type = parse_and_remove_chunked(
                        &mut body_headers.transfer_encoding,
                    );
                }
                // 5. Content-Encoding
                if key.eq_ignore_ascii_case(CE)
                    || key.eq_ignore_ascii_case(CONTENT_ENCODING)
                {
                    body_headers.content_encoding =
                        match_compression(header.value_as_str());
                }

                // 6. Content-Type
                if key.eq_ignore_ascii_case(CONTENT_TYPE)
                    && let Some((main_type, _)) =
                        header.value_as_str().split_once('/')
                {
                    body_headers.content_type =
                        Some(ContentType::from(main_type));
                }
            });

        // if TransferType is Unknown, and if content_encoding or transfer_encoding
        // or content_type is present, then set TransferType to Close
        if body_headers.transfer_type.is_none()
            && (body_headers.content_encoding.is_some()
                || body_headers.transfer_encoding.is_some()
                || body_headers.content_type.is_some())
        {
            body_headers.transfer_type = Some(TransferType::Close);
        }
        body_headers.sanitize()
    }
}

/* Description:
 *      Convert compression header values to Vec<ContentEncoding>.
 *
 * Steps:
 *      1. Split at ','
 *      2. Trim
 *      3. Filter_map with ContentEncoding::from()
 *      4. collect()
 *      5. If vec is not empty, return Some(vec), else return None.
 *
 */
pub fn match_compression(value: &str) -> Option<Vec<ContentEncoding>> {
    let encoding: Vec<ContentEncoding> = value
        .split(',')
        .map(|x| x.trim())
        .filter_map(|x| {
            if x.is_empty() {
                None
            } else {
                Some(ContentEncoding::from(x))
            }
        })
        .collect();
    Some(encoding)
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use mime::ContentType;

    use super::*;

    #[test]
    fn test_match_compression() {
        let data = "gzip, deflate, br, compress,";
        let result = match_compression(data);
        let verify = vec![
            ContentEncoding::Gzip,
            ContentEncoding::Deflate,
            ContentEncoding::Brotli,
            ContentEncoding::Compress,
        ];
        assert_eq!(result, Some(verify));
    }

    #[test]
    fn test_header_map_to_body_headers_cl_only() {
        let data = "Content-Length: 10\r\n\r\n";
        let buf = BytesMut::from(data);
        let header_map = HeaderMap::new(buf);
        let result = Option::<BodyHeader>::from(&header_map).unwrap();
        let verify = BodyHeader {
            transfer_type: Some(TransferType::ContentLength(10)),
            ..Default::default()
        };
        assert_eq!(result, verify);
    }

    #[test]
    fn test_header_map_to_body_headers_cl_invalid() {
        let data = "Content-Length: invalid\r\n\r\n";
        let buf = BytesMut::from(data);
        let header_map = HeaderMap::new(buf);
        let verify = BodyHeader {
            transfer_type: Some(TransferType::Close),
            ..Default::default()
        };
        let result = Option::<BodyHeader>::from(&header_map).unwrap();
        assert_eq!(result, verify);
    }

    #[test]
    fn test_header_map_to_body_headers_te_chunked() {
        let data = "Transfer-Encoding: chunked\r\n\r\n";
        let buf = BytesMut::from(data);
        let header_map = HeaderMap::new(buf);
        let verify = BodyHeader {
            transfer_type: Some(TransferType::Chunked),
            ..Default::default()
        };
        let result = Option::<BodyHeader>::from(&header_map).unwrap();
        assert_eq!(result, verify);
    }

    #[test]
    fn test_header_map_to_body_headers_content_length_and_chunked() {
        let data = "Content-Length: 20\r\nTransfer-Encoding: chunked\r\n\r\n";
        let buf = BytesMut::from(data);
        let header_map = HeaderMap::new(buf);
        let verify = BodyHeader {
            transfer_type: Some(TransferType::Chunked),
            ..Default::default()
        };
        let result = Option::<BodyHeader>::from(&header_map).unwrap();
        assert_eq!(result, verify);
    }

    #[test]
    fn test_header_map_to_body_headers_ct_only() {
        let data = "Content-Type: application/json\r\n\r\n";
        let buf = BytesMut::from(data);
        let header_map = HeaderMap::new(buf);
        let verify = BodyHeader {
            content_type: Some(ContentType::Application),
            transfer_type: Some(TransferType::Close),
            ..Default::default()
        };
        let result = Option::<BodyHeader>::from(&header_map).unwrap();
        assert_eq!(result, verify);
    }

    #[test]
    fn test_header_map_to_body_headers_ce_only() {
        let data = "Content-Encoding: gzip\r\n\r\n";
        let buf = BytesMut::from(data);
        let header_map = HeaderMap::new(buf);
        let verify = BodyHeader {
            content_encoding: Some(vec![ContentEncoding::Gzip]),
            transfer_type: Some(TransferType::Close),
            ..Default::default()
        };
        let result = Option::<BodyHeader>::from(&header_map).unwrap();
        assert_eq!(result, verify);
    }
}
