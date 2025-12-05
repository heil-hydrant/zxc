use std::borrow::Cow;
use std::str::{self};

use bytes::BytesMut;

use super::{InfoLine, InfoLineError};
use crate::abnf::OWS;

// Request Info Line
#[derive(Debug)]
pub struct Request {
    method: BytesMut,  // Method + Space
    uri: BytesMut,     //  Uri
    version: BytesMut, // Space + Version + CRLF
}

/* Steps:
 *      1. Find first OWS
 *      2. Call split_to(index)
 *      3. Find second OWS
 *      4. Call split_to(index)
 *      5. Return first, second, remaining (contains CRLF).
 *
 * Error:
 *      InfoLineError::FirstOWS     [1]
 *      InfoLineError::SecondOWS    [2]
 */

impl InfoLine for Request {
    fn build_infoline(mut data: BytesMut) -> Result<Request, InfoLineError> {
        let mut index = data
            .iter()
            .position(|&x| x == OWS as u8)
            .ok_or(InfoLineError::FirstOWS(
                String::from_utf8_lossy(&data).to_string(),
            ))?;
        let method = data.split_to(index + 1);
        // 2. Second OWS
        index = data
            .iter()
            .position(|&x| x == OWS as u8)
            .ok_or(InfoLineError::SecondOWS(
                String::from_utf8_lossy(&data).to_string(),
            ))?;
        let uri = data.split_to(index);
        Ok(Request {
            method,
            uri,
            version: data,
        })
    }

    fn into_data(mut self) -> BytesMut {
        self.uri.unsplit(self.version);
        self.method.unsplit(self.uri);
        self.method
    }
}

impl Request {
    // Fix
    // https://doc.rust-lang.org/std/primitive.slice.html#method.trim_ascii_end
    pub fn method(&self) -> &[u8] {
        self.method.split_last().unwrap().1
    }

    pub fn method_raw(&self) -> &BytesMut {
        &self.method
    }

    pub fn set_method_raw(&mut self, method: BytesMut) {
        self.method = method;
    }

    // Uri Related
    pub fn uri_as_mut(&mut self) -> &mut BytesMut {
        &mut self.uri
    }

    pub fn uri_as_string(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infoline_request_basic() {
        let req = "GET /echo HTTP/1.1\r\n";
        let buf = BytesMut::from(req);
        let verify = buf[0..20].to_owned();
        let verify_ptr = buf[0..20].as_ptr_range();
        let request = Request::build_infoline(buf).unwrap();
        assert_eq!(request.method(), b"GET");
        assert_eq!(request.uri_as_string(), "/echo");
        assert_eq!(request.version, " HTTP/1.1\r\n");
        let toverify = request.into_data();
        assert_eq!(verify_ptr, toverify.as_ptr_range());
        assert_eq!(toverify, verify);
    }

    #[test]
    fn test_infoline_request_connect() {
        let req = "CONNECT www.google.com:443 HTTP/1.1\r\n";
        let buf = BytesMut::from(req);
        let verify_ptr = buf[..37].as_ptr_range();
        let verify = buf.clone();
        match Request::build_infoline(buf) {
            Ok(header) => {
                assert_eq!(header.method, "CONNECT ");
                assert_eq!(header.uri, "www.google.com:443");
                assert_eq!(header.version, " HTTP/1.1\r\n");
                let assembled = header.into_data();
                assert_eq!(assembled, verify);
                assert_eq!(verify_ptr, assembled.as_ptr_range());
            }
            _ => {
                panic!();
            }
        }
    }

    #[test]
    fn test_infoline_request_http() {
        let req = "GET http://www.google.com/ HTTP/1.1\r\n";
        let buf = BytesMut::from(req);
        let verify_ptr = buf[..].as_ptr_range();
        let verify = buf.clone();
        match Request::build_infoline(buf) {
            Ok(header) => {
                assert_eq!(header.method, "GET ");
                assert_eq!(header.uri, "http://www.google.com/");
                assert_eq!(header.version, " HTTP/1.1\r\n");
                let assembled = header.into_data();
                assert_eq!(assembled, verify);
                assert_eq!(verify_ptr, assembled.as_ptr_range());
            }
            _ => {
                panic!();
            }
        }
    }

    #[test]
    fn test_infoline_request_http_port() {
        let req = "GET http://www.google.com:8080/ HTTP/1.1\r\n";
        let buf = BytesMut::from(req);
        let verify_ptr = buf[..].as_ptr_range();
        let verify = buf.clone();
        match Request::build_infoline(buf) {
            Ok(header) => {
                assert_eq!(header.method, "GET ");
                assert_eq!(header.uri, "http://www.google.com:8080/");
                assert_eq!(header.version, " HTTP/1.1\r\n");
                let assembled = header.into_data();
                assert_eq!(assembled, verify);
                assert_eq!(verify_ptr, assembled.as_ptr_range());
            }
            _ => {
                panic!();
            }
        }
    }
}
