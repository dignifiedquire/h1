use async_std::prelude::*;
use async_std::sync::RwLock;
use async_std::{io, net, task};
use futures::io::AsyncRead;
use path_tree::PathTree;
use std::pin::Pin;

use crate::{Handler, Middleware, Request, Response};

pub type Params<'a> = Vec<(&'a str, &'a str)>;

pub struct App {
    pub(crate) listener: net::TcpListener,
    pub(crate) router: std::sync::Arc<RwLock<PathTree<Box<dyn Handler>>>>,
    pub(crate) middleware: std::sync::Arc<RwLock<Vec<Box<dyn Middleware>>>>,
}

impl App {
    pub async fn run(&self) -> io::Result<()> {
        let mut incoming = self.listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let router = self.router.clone();
            let middleware = self.middleware.clone();

            task::spawn(async move {
                let router = router.read().await;
                let middleware = middleware.read().await;
                // TODO: what about errors?

                let (reader, writer) = &mut (&stream, &stream);

                let response = process(reader, &*router, &*middleware).await.unwrap();

                // Write header data
                let header = response.header_encoded();
                writer.write_all(header.as_bytes()).await.unwrap();

                // Write body
                writer.write_all(response.body_encoded()).await.unwrap();
            });
        }

        Ok(())
    }
}

use tendril::{SendTendril, Tendril};
type ByteTendril = Tendril<tendril::fmt::Bytes, tendril::Atomic>;

async fn process(
    stream: &mut &net::TcpStream,
    router: &PathTree<Box<dyn Handler>>,
    middleware: &[Box<dyn Middleware>],
) -> io::Result<Response> {
    let mut buf: ByteTendril = Tendril::from_slice(&[0u8; 1024][..]);

    while read_to_tendril(stream, &mut buf).await? > 0 {
        let buf = buf.subtendril(0, buf.len32()).into_send();
        if let Some(req) = parse_request(buf)? {
            let resp = process_inner(req, router, middleware).await?;
            return Ok(resp);
        }
    }

    panic!("Failed to read a response")
}

async fn read_to_tendril<T>(stream: &mut T, buf: &mut ByteTendril) -> io::Result<usize>
where
    T: Unpin + AsyncRead,
{
    let mut stream = Pin::new(stream);
    let read = futures::future::poll_fn(|cx| stream.as_mut().poll_read(cx, buf)).await?;
    Ok(read)
}

fn parse_request(buf: SendTendril<tendril::fmt::Bytes>) -> io::Result<Option<Request>> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut buf: ByteTendril = buf.into();

    let (method, path, version, headers, amt) = {
        let mut r = httparse::Request::new(&mut headers);
        let status = r.parse(&buf).map_err(|e| {
            let msg = format!("failed to parse http request: {:?}", e);
            io::Error::new(io::ErrorKind::Other, msg)
        })?;

        let amt = match status {
            httparse::Status::Complete(amt) => amt,
            httparse::Status::Partial => return Ok(None),
        };

        // TODO: stop allocating, but that requires a parser that understands
        // Tendrils, not just byte slices.
        (
            r.method.unwrap().as_bytes().to_vec(),
            r.path.unwrap().as_bytes().to_vec(),
            r.version.unwrap(),
            r.headers
                .iter()
                .map(|h| (h.name.as_bytes().to_vec(), h.value.to_vec()))
                .collect(),
            amt,
        )
    };
    buf.pop_back(buf.len32() - amt as u32);

    Ok(Some(Request {
        method,
        path,
        version,
        headers,
        data: buf[..amt].to_vec(),
    }))
}

async fn process_inner(
    mut req: Request,
    router: &PathTree<Box<dyn Handler>>,
    middleware: &[Box<dyn Middleware>],
) -> io::Result<Response> {
    let path = format!("/{}/{}", req.method(), req.path());

    for mid in middleware {
        mid.call(&mut req).await?;
    }

    match router.find(&path) {
        Some((handler, params)) => {
            let resp = handler.call(req, params).await?;
            Ok(resp)
        }
        None => {
            let mut resp = Response::default();
            resp.status_code(404, "Not Found");

            Ok(resp)
        }
    }
}
