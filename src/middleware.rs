use async_std::io;
use async_trait::async_trait;

use crate::Request;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn call(&self, request: &mut Request<'_>) -> io::Result<()>;
}

/// Log all requests.
pub struct Logger {}

impl Default for Logger {
    fn default() -> Self {
        femme::pretty::Logger::new()
            .start(log::LevelFilter::Info)
            .unwrap();

        Logger {}
    }
}

#[async_trait]
impl Middleware for Logger {
    async fn call(&self, request: &mut Request<'_>) -> io::Result<()> {
        log::info!("[{}] {}", request.method(), request.path());
        Ok(())
    }
}
