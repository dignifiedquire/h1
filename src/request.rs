use std::str;

#[derive(Default)]
pub struct Request {
    pub(crate) method: Vec<u8>,
    pub(crate) path: Vec<u8>,
    pub(crate) version: u8,
    // TODO: use a small vec to avoid this unconditional allocation
    pub(crate) headers: Vec<(Vec<u8>, Vec<u8>)>,
    pub(crate) data: Vec<u8>,
}

impl Request {
    pub fn method(&self) -> &str {
        str::from_utf8(&self.method).unwrap()
    }

    pub fn path(&self) -> &str {
        str::from_utf8(&self.path).unwrap()
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn headers(&self) -> &[(Vec<u8>, Vec<u8>)] {
        &self.headers
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
