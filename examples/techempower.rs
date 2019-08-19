#![feature(async_await)]

use async_std::{io, task};
use async_trait::async_trait;

use h1::{Handler, Params, Request, Response, H1};

pub struct PlaintextRoute;

#[async_trait]
impl Handler for PlaintextRoute {
    async fn call(&self, _request: Request<'_>, _params: Params<'_>) -> io::Result<Response> {
        let mut resp = Response::default();
        resp.header("Content-Type", "text/plain")
            .body("Hello, World!");

        Ok(resp)
    }
}

pub struct JsonRoute;

#[async_trait]
impl Handler for JsonRoute {
    async fn call(&self, _request: Request<'_>, _params: Params<'_>) -> io::Result<Response> {
        let mut resp = Response::default();
        let json = serde_json::to_string(&serde_json::json!({
            "message": "Hello, World!"
        }))
        .unwrap();

        resp.header("Content-Type", "application/json").body(&json);

        Ok(resp)
    }
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let app = H1::default()
            .get("/json", JsonRoute)
            .get("/plaintext", PlaintextRoute)
            .listen("localhost:3000")
            .await?;

        println!("Listening on http://localhost:3000");

        app.run().await?;

        Ok(())
    })
}
