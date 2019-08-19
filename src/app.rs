use async_std::prelude::*;
use async_std::sync::RwLock;
use async_std::{io, net, task};
use futures::future::Either;
use lazy_static::lazy_static;
use path_tree::PathTree;
use veryfast::pool::Pool;

use crate::request::decode;
use crate::{Handler, Middleware, Request, Response};

pub type Params<'a> = Vec<(&'a str, &'a str)>;

lazy_static! {
    static ref POOL: Pool<Vec<u8>> = Pool::with_params(true);
}

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

async fn process(
    stream: &mut &net::TcpStream,
    router: &PathTree<Box<dyn Handler>>,
    middleware: &[Box<dyn Middleware>],
) -> io::Result<Response> {
    let mut buf = POOL.push(vec![0u8; 4096]);
    let mut total_read = 0;

    loop {
        let read = stream.read(&mut buf).await?;
        if read == 0 {
            break;
        }
        total_read += read;

        match decode(buf)? {
            Either::Left(req) => {
                dbg!(&req);
                let resp = process_inner(req, router, middleware).await?;
                return Ok(resp);
            }
            Either::Right(old_buf) => {
                buf = old_buf;
                // Grow the buffer if we need to
                if total_read >= buf.len() {
                    let l = buf.len();
                    buf.resize(l + 256, 0);
                }
            }
        }
    }

    panic!("Failed to read a response")
}

async fn process_inner(
    mut req: Request<'_>,
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
