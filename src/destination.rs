use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use log::{info, error,LevelFilter};
use std::sync::Arc;
use std::io::Write;
use tokio::sync::Mutex;
use chrono::Local;
use env_logger::Builder;

// server destination will listen
const ADDR: &str = "127.0.0.1:9090";


pub async fn run_destination(messages: Arc<Mutex<Vec<String>>>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(ADDR).await?;
    run_destination_with_listener(messages, listener).await
}

// pub async fn run_destination(messages: Arc<Mutex<Vec<String>>>) -> Result<(), Box<dyn std::error::Error>> {
pub async fn run_destination_with_listener(messages: Arc<Mutex<Vec<String>>>, listener: TcpListener) -> Result<(), Box<dyn std::error::Error>> {    
    info!("Starting destination server on {}", ADDR);

    // tcp listener bound to address
    let listener = TcpListener::bind(ADDR).await?;

    loop {

        // accepts a new client connection
        let (mut socket, addr) = listener.accept().await?;

        // logging
        info!("Received connection from: {}", addr);

        // clones the arc for shared mssgs
        let messages = Arc::clone(&messages);

        // spawns a new task to handle new client
        tokio::spawn(async move {
            let mut buffer = String::new();
            loop {
                match socket.read_to_string(&mut buffer).await {
                    Ok(0) => break,
                    // n-bytes are received successfully
                    // stores each message in shared vector
                    Ok(n) => {
                        info!("Received {} bytes", n);
                        let mut messages = messages.lock().await;
                        for line in buffer.lines() {
                            info!("Destination received: {}", line);
                            messages.push(line.to_string());
                        }
                        buffer.clear();
                    }
                    // error handling
                    Err(e) => {
                        error!("Failed to read from socket: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

// main tokio function
#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // initialising logger
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    // creates a shared vector to store mssg
    let messages = Arc::new(Mutex::new(Vec::new()));

    // runs destination server
    run_destination(messages).await
}