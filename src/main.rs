use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use log::{info, error};
use std::sync::Arc;
use tokio::sync::Mutex;

const ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialise the logger to log results
    env_logger::init();

    info!("Starting TCP log server on {}", ADDR);

    // TCP listener
    let listener = TcpListener::bind(ADDR).await?;

    // shared buffer for holding messages
    let buffer = Arc::new(Mutex::new(Vec::new()));

    loop {
        // accept incoming client connection
        let (socket, addr) = listener.accept().await?;
        info!("New client connected: {}", addr);

         
        let buffer = Arc::clone(&buffer);

        
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, buffer).await {
                error!("Error handling client {}: {}", addr, e);
            }
        });
    }
}

async fn handle_client(mut socket: TcpStream, buffer: Arc<Mutex<Vec<String>>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut message = String::new();

    loop {
        // reading data from the socket
        let mut buf = [0; 1024];
        let n = socket.read(&mut buf).await?;

        if n == 0 {
            // close connection
            return Ok(());
        }

        // convert the message to string
        // and add it to message
        message.push_str(&String::from_utf8_lossy(&buf[..n]));

        // checking if message has ended
        if message.ends_with('\n') {
            // remove new line
            message.pop();

            // add the message to the buffer
            let mut buffer = buffer.lock().await;
            buffer.push(message.clone());

            info!("Received message: {}", message);

            // check if we need to flush the buffer
            if buffer.len() >= 100 {
                flush_buffer(&mut buffer).await;
            }

            // clear the message for next incoming message
            message.clear();
        }
    }
}

async fn flush_buffer(buffer: &mut Vec<String>) {
    info!("Flushing buffer with {} messages", buffer.len());
    // TODO
    buffer.clear();
}