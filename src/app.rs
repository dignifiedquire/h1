use async_std::io::BufReader;
use async_std::prelude::*;
use async_std::sync::RwLock;
use async_std::{io, net, task};
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

                match process(&stream, &*router, &*middleware).await {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("ERROR: {:?}", err);
                    }
                }
            });
        }

        Ok(())
    }
}

async fn process(
    mut stream: &net::TcpStream,
    router: &PathTree<Box<dyn Handler>>,
    middleware: &[Box<dyn Middleware>],
) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut buf = POOL.push(Vec::new());

    loop {
        let read = reader.read_until(b'\n', &mut buf).await?;
        let end = buf.len() - 1;
        if read == 0 {
            break;
        } else if end >= 3 && buf[end - 3..=end] == [13, 10, 13, 10] {
            // bounds check, then consecutive 'CRLF' check
            break;
        }
    }

    let req = decode(buf)?;
    let response = process_inner(req, router, middleware).await?;

    // Write header data
    let header = response.header_encoded();
    stream.write_all(header.as_bytes()).await?;

    // Write body
    stream.write_all(response.body_encoded()).await?;
    Ok(())
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
