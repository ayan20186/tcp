use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use log::{info, error, debug};
use std::sync::Arc;
use tokio::sync::Mutex;
use env_logger::Builder;
use chrono::Local;
use log::LevelFilter;
use std::io::Write;

pub async fn run_destination(messages: Arc<Mutex<Vec<String>>>, addr: String) -> Result<(), Box<dyn std::error::Error>> {
    // Log the start of the destination server
    info!("Starting destination server on {}", addr);

    // Bind the TCP listener to the specified address
    let listener = TcpListener::bind(&addr).await?;

    // Main server loop
    loop {
        tokio::select! {
            // Wait for incoming connections
            result = listener.accept() => {
                match result {
                    Ok((mut socket, addr)) => {
                        info!("Received connection from: {}", addr);
                        let messages = Arc::clone(&messages);
                        // Spawn a new task to handle the connection
                        tokio::spawn(async move {
                            let mut buffer = String::new();
                            // Read from the socket in a loop
                            loop {
                                match socket.read_to_string(&mut buffer).await {
                                    Ok(0) => break, // Connection closed
                                    Ok(n) => {
                                        debug!("Received {} bytes", n);
                                        // Lock the shared messages vector
                                        let mut messages = messages.lock().await;
                                        // Process each line in the received data
                                        for line in buffer.lines() {
                                            debug!("Destination received: {}", line);
                                            messages.push(line.to_string());
                                        }
                                        info!("Destination total messages: {}", messages.len());
                                        buffer.clear();
                                    }
                                    Err(e) => {
                                        error!("Failed to read from socket: {}", e);
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
            // Handle Ctrl+C signal for graceful shutdown
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
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
    // Create a shared vector to store received messages
    let messages = Arc::new(Mutex::new(Vec::new()));
    // Run the destination server on localhost:9090
    run_destination(messages, "127.0.0.1:9090".to_string()).await
}