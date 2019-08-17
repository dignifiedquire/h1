#![feature(async_await)]

use async_std::sync::RwLock;
use async_std::{io, net};
use std::net::ToSocketAddrs;
use std::str;
use std::sync::Arc;

use async_trait::async_trait;
use path_tree::PathTree;

mod app;
mod date;
mod request;
mod response;

pub use crate::app::*;
pub use crate::request::*;
pub use crate::response::*;

#[async_trait]
pub trait Handler: Send + Sync {
    async fn call(&self, request: Request, params: Params<'_>) -> io::Result<Response>;
}

#[derive(Default)]
pub struct H1 {
    router: PathTree<Box<dyn Handler>>,
}

impl H1 {
    pub fn get<H: Handler + Sized + 'static>(mut self, route: impl AsRef<str>, handle: H) -> Self {
        self.router
            .insert(&format!("/GET/{}", route.as_ref()), Box::new(handle));
        self
    }

    pub async fn listen<A: ToSocketAddrs>(self, addrs: A) -> io::Result<App> {
        let listener = net::TcpListener::bind(addrs).await?;
        Ok(App {
            listener,
            router: Arc::new(RwLock::new(self.router)),
        })
    }
}
