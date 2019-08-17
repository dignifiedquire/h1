use async_std::prelude::*;
use async_std::sync::RwLock;
use async_std::{io, net, task};
use path_tree::PathTree;

use crate::{Handler, Request, Response};

pub type Params<'a> = Vec<(&'a str, &'a str)>;

pub struct App {
    pub(crate) listener: net::TcpListener,
    pub(crate) router: std::sync::Arc<RwLock<PathTree<Box<dyn Handler>>>>,
}

impl App {
    pub async fn run(&self) -> io::Result<()> {
        let mut incoming = self.listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let router = self.router.clone();

            task::spawn(async move {
                let router = router.read().await;
                // TODO: what about errors?

                let (reader, writer) = &mut (&stream, &stream);

                let response = process(reader, &*router).await.unwrap();

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

async fn process(
    stream: &mut &net::TcpStream,
    router: &PathTree<Box<dyn Handler>>,
) -> io::Result<Response> {
    let mut buf = vec![0u8; 1024];

    while stream.read(&mut buf).await? > 0 {
        if let Some(resp) = process_inner(&buf, router).await? {
            return Ok(resp);
        }
    }

    panic!("Failed to read a response")
}

async fn process_inner(
    buf: &[u8],
    router: &PathTree<Box<dyn Handler>>,
) -> io::Result<Option<Response>> {
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

    let (method, path, version, headers) = (
        r.method.unwrap().as_bytes().to_vec(),
        r.path.unwrap().as_bytes().to_vec(),
        r.version.unwrap(),
        r.headers
            .iter()
            .map(|h| (h.name.as_bytes().to_vec(), h.value.to_vec()))
            .collect(),
    );

    let req = Request {
        method,
        path,
        version,
        headers,
        data: buf[..amt].to_vec(),
    };

    let path = format!("/{}/{}", req.method(), req.path());
    dbg!(&path);

    match router.find(&path) {
        Some((handler, params)) => {
            let resp = handler.call(req, params).await?;
            Ok(Some(resp))
        }
        None => {
            let mut resp = Response::default();
            resp.status_code(404, "Not Found");

            Ok(Some(resp))
        }
    }
}
