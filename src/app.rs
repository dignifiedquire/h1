use async_std::sync::RwLock;
use async_std::{io, net};
use futures::io::AsyncBufReadExt;
use futures::stream::TryStreamExt;
use path_tree::PathTree;

use crate::request::RequestsCodec;
use crate::{Handler, Middleware, Request, Response};

pub type Params<'a> = Vec<(&'a str, &'a str)>;

pub struct App {
    pub(crate) listener: net::TcpListener,
    pub(crate) router: std::sync::Arc<RwLock<PathTree<Box<dyn Handler>>>>,
    pub(crate) middleware: std::sync::Arc<RwLock<Vec<Box<dyn Middleware>>>>,
}

impl App {
    pub async fn run(&self) -> io::Result<()> {
        self.listener
            .incoming()
            .try_for_each_concurrent(/* limit */ 1000, |stream| {
                let router = self.router.clone();
                let middleware = self.middleware.clone();

                async move {
                    let router = router.read().await;
                    let middleware = middleware.read().await;

                    let (reader, writer) = &mut (&stream, &stream);

                    let response = futures_codec::FramedRead::new(reader, RequestsCodec {})
                        .and_then(|req| process_middleware(req, &*middleware))
                        .and_then(|req| process_routes(req, &*router));

                    // This is very sad and breaks our whole flow
                    pin_utils::pin_mut!(response);

                    response.into_async_read().copy_buf_into(writer).await?;

                    Ok(())
                }
            })
            .await?;

        Ok(())
    }
}

async fn process_middleware(
    mut req: Request,
    middleware: &[Box<dyn Middleware>],
) -> io::Result<Request> {
    for mid in middleware {
        mid.call(&mut req).await?;
    }

    Ok(req)
}

async fn process_routes(req: Request, router: &PathTree<Box<dyn Handler>>) -> io::Result<Vec<u8>> {
    let path = format!("/{}/{}", req.method(), req.path());
    log::trace!("{}", &path);

    let resp = match router.find(&path) {
        Some((handler, params)) => handler.call(req, params).await?,
        None => {
            let mut resp = Response::default();
            resp.status_code(404, "Not Found");
            resp
        }
    };

    Ok(resp.finalize())
}
