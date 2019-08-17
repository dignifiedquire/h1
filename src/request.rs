use std::str;

use async_std::io;
use bytes::BytesMut;
use futures_codec::Decoder;

type Slice = (usize, usize);

#[derive(Default)]
pub struct Request {
    pub(crate) method: Slice,
    pub(crate) path: Slice,
    pub(crate) version: u8,
    // TODO: use a small vec to avoid this unconditional allocation
    pub(crate) headers: Vec<(Slice, Slice)>,
    pub(crate) data: BytesMut,
}

impl Request {
    pub fn method(&self) -> &str {
        str::from_utf8(self.slice(&self.method)).unwrap()
    }

    pub fn path(&self) -> &str {
        str::from_utf8(self.slice(&self.path)).unwrap()
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn headers(&self) -> RequestHeaders {
        RequestHeaders {
            headers: self.headers.iter(),
            req: self,
        }
    }

    fn slice(&self, slice: &Slice) -> &[u8] {
        &self.data[slice.0..slice.1]
    }

    // TODO: implement body handling
}

pub struct RequestHeaders<'req> {
    headers: std::slice::Iter<'req, (Slice, Slice)>,
    req: &'req Request,
}

impl<'req> Iterator for RequestHeaders<'req> {
    type Item = (&'req str, &'req [u8]);

    fn next(&mut self) -> Option<(&'req str, &'req [u8])> {
        self.headers.next().map(|&(ref a, ref b)| {
            let a = self.req.slice(a);
            let b = self.req.slice(b);
            (str::from_utf8(a).unwrap(), b)
        })
    }
}

pub(crate) struct RequestsCodec {}

impl Decoder for RequestsCodec {
    type Item = Request;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut bytes::BytesMut) -> io::Result<Option<Self::Item>> {
        let mut headers = [httparse::EMPTY_HEADER; 16];

        let mut r = httparse::Request::new(&mut headers);
        let status = r.parse(&buf).map_err(|e| {
            let msg = format!("failed to parse http request: {:?}", e);
            io::Error::new(io::ErrorKind::Other, msg)
        })?;

        let amt = match status {
            httparse::Status::Complete(amt) => amt,
            httparse::Status::Partial => return Ok(None),
        };

        let to_slice = |a: &[u8]| {
            let start = a.as_ptr() as usize - buf.as_ptr() as usize;
            assert!(start < buf.len());
            (start, start + a.len())
        };

        let (method, path, version, headers) = (
            to_slice(r.method.unwrap().as_bytes()),
            to_slice(r.path.unwrap().as_bytes()),
            r.version.unwrap(),
            r.headers
                .iter()
                .map(|h| (to_slice(h.name.as_bytes()), to_slice(h.value)))
                .collect(),
        );

        Ok(Some(Request {
            method,
            path,
            version,
            headers,
            data: buf.split_to(amt),
        }))
    }
}
