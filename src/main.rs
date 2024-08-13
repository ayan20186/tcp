use std::error::Error;

mod server;
//mod elasticsearch_integration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    server::run_server("127.0.0.1:8080".to_string(), "127.0.0.1:9090".to_string()).await
}