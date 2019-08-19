#![feature(async_await)]

#[macro_use]
extern crate rental;

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

pub mod middleware;

pub use crate::app::*;
pub use crate::middleware::Middleware;
pub use crate::request::*;
pub use crate::response::*;

#[async_trait]
pub trait Handler: Send + Sync {
    async fn call(&self, request: Request<'_>, params: Params<'_>) -> io::Result<Response>;
}

#[derive(Default)]
pub struct H1 {
    router: PathTree<Box<dyn Handler>>,
    middleware: Vec<Box<dyn Middleware>>,
}

impl H1 {
    pub fn get<H: Handler + Sized + 'static>(mut self, route: impl AsRef<str>, handle: H) -> Self {
        self.router
            .insert(&format!("/GET/{}", route.as_ref()), Box::new(handle));
        self
    }

    /// Add middleware.
    pub fn using<H: Middleware + Sized + 'static>(mut self, middleware: H) -> Self {
        self.middleware.push(Box::new(middleware));
        self
    }

    /// Start listening on this address.
    pub async fn listen<A: ToSocketAddrs>(self, addrs: A) -> io::Result<App> {
        let listener = net::TcpListener::bind(addrs).await?;
        let Self {
            middleware, router, ..
        } = self;

        Ok(App {
            listener,
            middleware: Arc::new(RwLock::new(middleware)),
            router: Arc::new(RwLock::new(router)),
        })
    }
}
