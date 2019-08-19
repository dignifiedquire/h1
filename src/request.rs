use std::{fmt, str};

use async_std::io;
use futures::future::Either;
use veryfast::pool::Object;

pub use rentals::Request;

rental! {
    pub mod rentals {
        use super::*;

        #[rental]
        pub struct Request<'a> {
            data: Object<'a, Vec<u8>>,
            request: InnerRequest<'data>,
        }
    }
}

impl<'a> fmt::Debug for Request<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<HTTP Request {} {} {:?}>",
            self.method(),
            self.path(),
            self.headers()
                .iter()
                .map(|(a, b)| (str::from_utf8(a).unwrap(), str::from_utf8(b).unwrap()))
                .collect::<Vec<_>>()
        )
    }
}

impl<'a> Request<'a> {
    pub fn method(&self) -> &str {
        self.ref_rent(|req| req.method())
    }

    pub fn path(&self) -> &str {
        self.ref_rent(|req| req.path())
    }

    pub fn version(&self) -> u8 {
        self.rent(|req| req.version())
    }

    pub fn headers(&self) -> &Vec<(&[u8], &[u8])> {
        // FIX THIS
        unsafe { self.all_erased().request.headers() }
    }
}

#[derive(Debug)]
pub struct InnerRequest<'data> {
    method: &'data [u8],
    path: &'data [u8],
    version: u8,
    // TODO: use a small vec to avoid this unconditional allocation
    headers: Vec<(&'data [u8], &'data [u8])>,
}

impl<'a> InnerRequest<'a> {
    fn method(&self) -> &str {
        // safe because it was a valid string upon construction
        unsafe { str::from_utf8_unchecked(&self.method) }
    }

    fn path(&self) -> &str {
        // safe because it was a valid string upon construction
        unsafe { str::from_utf8_unchecked(&self.path) }
    }

    fn version(&self) -> u8 {
        self.version
    }

    pub fn headers(&self) -> &Vec<(&[u8], &[u8])> {
        &self.headers
    }
}

pub fn decode(buf: Object<Vec<u8>>) -> io::Result<Either<Request, Object<Vec<u8>>>> {
    match Request::try_new(buf, |buf| {
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut r = httparse::Request::new(&mut headers);
        let status = match r.parse(&buf) {
            Ok(s) => s,
            Err(err) => {
                let msg = format!("failed to parse http request: {:?}", err);
                return Err(ParseError::Failure(io::Error::new(
                    io::ErrorKind::Other,
                    msg,
                )));
            }
        };

        match status {
            httparse::Status::Complete(amt) => amt,
            httparse::Status::Partial => return Err(ParseError::Partial),
        };

        Ok(InnerRequest {
            method: r.method.unwrap().as_bytes(),
            path: r.path.unwrap().as_bytes(),
            version: r.version.unwrap(),
            headers: r
                .headers
                .iter()
                .map(|h| (h.name.as_bytes(), h.value))
                .collect(),
        })
    }) {
        Ok(req) => Ok(Either::Left(req)),
        Err(rental::RentalError(ParseError::Partial, buf)) => Ok(Either::Right(buf)),
        Err(rental::RentalError(ParseError::Failure(err), _buf)) => Err(err),
    }
}

enum ParseError {
    Failure(io::Error),
    Partial,
}
