use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{interval, Duration};
use tokio::sync::mpsc;
use log::{info, error, warn, LevelFilter};
use chrono::Local;
use env_logger::Builder;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::io::Write;

// Addr where server will listen on
const ADDR: &str = "127.0.0.1:8080";  
// Destination server where buffer will be stored          
const UNDER_ADDR: &str = "127.0.0.1:9090"; 
// Flushing interval 10 secs           
const FLUSH_INTERVAL: Duration = Duration::from_secs(10);
// Max Buffer size
const BATCH_SIZE: usize = 100;


pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(ADDR).await?;
    run_server_with_listener(listener, UNDER_ADDR.to_string()).await
}


// pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
pub async fn run_server_with_listener(listener: TcpListener, destination_addr: String) -> Result<(), Box<dyn std::error::Error>> {
    // log message
    info!("Starting TCP log server on {}", ADDR);

    // TCP listener bound to server
    let listener = TcpListener::bind(ADDR).await?;

    // Mutable vector to store messages
    let buffer = Arc::new(Mutex::new(Vec::new()));

    // channel for sending flush signal
    let (flush_sender, flush_receiver) = mpsc::channel(100);

    // spawning task to handle buffer flushing
    tokio::spawn(buffer_flusher(Arc::clone(&buffer), flush_receiver));

    let timer_flush_sender = flush_sender.clone();

    // sends a flush signal at regular intervals
    tokio::spawn(async move {
        let mut interval = interval(FLUSH_INTERVAL);
        loop {
            interval.tick().await;
            if timer_flush_sender.send(()).await.is_err() {
                error!("Failed to send flush signal");
                break;
            }
        }
    });

    loop {

        // accepts a new client connection
        let (mut socket, addr) = listener.accept().await?;

        //logging
        info!("New client connected: {}", addr);

        let buffer = Arc::clone(&buffer);
        let flush_sender = flush_sender.clone();

        // a new task, to handle new client connections
        tokio::spawn(async move {
            // buffer to store message
            let mut message = String::new();
            loop {
                // buffer for reading from socket
                let mut buf = [0; 1024];

                match socket.read(&mut buf).await {

                    Ok(0) => break,
                    // if n-bytes are read, then we append the data
                    // to the message buffer
                    Ok(n) => {
                        message.push_str(&String::from_utf8_lossy(&buf[..n]));
                        while let Some(index) = message.find('\n') {
                            let complete_message = message[..index].to_string();
                            message = message[index + 1..].to_string();

                            // adds message to the shared buffer
                            let mut buffer = buffer.lock().await;
                            buffer.push(complete_message.clone());
                            info!("Received message: {}", complete_message);
                            // if buffer size reaches 100 messages
                            // we flush the buffer
                            if buffer.len() >= BATCH_SIZE {
                                drop(buffer);
                                if flush_sender.send(()).await.is_err() {
                                    error!("Failed to send flush signal");
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read from socket: {}", e);
                        break;
                    }
                }
            }
            info!("Client disconnected: {}", addr);
        });
    }
}

// function to flush the buffer when signal received
async fn buffer_flusher(buffer: Arc<Mutex<Vec<String>>>, mut flush_receiver: mpsc::Receiver<()>) {
    while flush_receiver.recv().await.is_some() {
        let mut buffer = buffer.lock().await;
        if !buffer.is_empty() {
            if let Err(e) = send_batch_to_destination(&buffer).await {
                error!("Failed to send batch to destination: {}", e);
            } else {
                info!("Flushed buffer with {} messages", buffer.len());
                buffer.clear();
            }
        }
    }
}

// function to send batch message to destination server/underlying server
async fn send_batch_to_destination(batch: &[String]) -> Result<(), Box<dyn std::error::Error>> {

    // connect to underlying server
    let mut stream = TcpStream::connect(UNDER_ADDR).await?;

    //write each message to the stream
    for message in batch {
        stream.write_all(message.as_bytes()).await?;
        stream.write_all(b"\n").await?;
    }
    stream.flush().await?;
    Ok(())
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    //initializing the logger
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

    //runs the server
    run_server().await
}