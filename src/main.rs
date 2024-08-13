use std::error::Error;
use env_logger::Builder;
use chrono::Local;
use log::LevelFilter;
use std::io::Write;

mod server;
//mod elasticsearch_integration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    //initializing the logger
    // Builder::new()
    //     .format(|buf, record| {
    //         writeln!(buf,
    //             "{} [{}] - {}",
    //             Local::now().format("%Y-%m-%d %H:%M:%S"),
    //             record.level(),
    //             record.args()
    //         )
    //     })
    //     .filter(None, LevelFilter::Info)
    //     .init();
    server::run_server("127.0.0.1:8080".to_string(), "127.0.0.1:9090".to_string()).await
}