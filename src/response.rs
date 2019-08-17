use std::fmt;

use crate::date;

#[derive(Default)]
pub struct Response {
    headers: Vec<(String, String)>,
    response: Vec<u8>,
    status_message: StatusMessage,
}

enum StatusMessage {
    Ok,
    Custom(u32, String),
}

impl Default for StatusMessage {
    fn default() -> Self {
        StatusMessage::Ok
    }
}

impl Response {
    pub fn status_code(&mut self, code: u32, message: &str) -> &mut Response {
        self.status_message = StatusMessage::Custom(code, message.to_string());
        self
    }

    pub fn header(&mut self, name: &str, val: &str) -> &mut Response {
        self.headers.push((name.to_string(), val.to_string()));
        self
    }

    pub fn body(&mut self, s: &str) -> &mut Response {
        self.response = s.as_bytes().to_vec();
        self
    }

    pub fn body_bytes(&mut self, b: &[u8]) -> &mut Response {
        self.response = b.to_vec();
        self
    }

    pub(crate) fn header_encoded(&self) -> String {
        // TODO: do not allocate, and make a proper streaming writer thingy, when I am not tired anymore.
        let mut data = format!(
            "\
             HTTP/1.1 {}\r\n\
             Server: Example\r\n\
             Content-Length: {}\r\n\
             Date: {}\r\n\
             ",
            self.status_message,
            self.response.len(),
            date::now()
        );

        for &(ref k, ref v) in &self.headers {
            data += k;
            data += ": ";
            data += v;
            data += "\r\n";
        }

        data += "\r\n";

        data
    }

    pub(crate) fn body_encoded(&self) -> &[u8] {
        self.response.as_slice()
    }
}

impl fmt::Display for StatusMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StatusMessage::Ok => f.pad("200 OK"),
            StatusMessage::Custom(c, ref s) => write!(f, "{} {}", c, s),
        }
    }
}
