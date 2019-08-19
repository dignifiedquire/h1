#![feature(async_await)]

use async_std::{io, task};
use async_trait::async_trait;

use h1::{middleware, Handler, Params, Request, Response, H1};

pub struct GetRoot;

#[async_trait]
impl Handler for GetRoot {
    async fn call(&self, _request: Request<'_>, _params: Params<'_>) -> io::Result<Response> {
        let mut resp = Response::default();
        resp.body("Hello, world!");

        Ok(resp)
    }
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let app = H1::default()
            .using(middleware::Logger::default())
            .get("/", GetRoot)
            .listen("localhost:3000")
            .await?;

        println!("Listening on http://localhost:3000");

        app.run().await?;

        Ok(())
    })
}
