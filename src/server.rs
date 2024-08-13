use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{interval, Duration};
use tokio::sync::{mpsc, Mutex};
use log::{info, error, warn, debug};
use std::sync::Arc;
use std::error::Error;
use std::fmt;
use env_logger::Builder;
use chrono::Local;
use log::LevelFilter;
use std::io::Write;
use std::sync::Once;


static INIT: Once = Once::new();

// Flushing interval 10 secs
const FLUSH_INTERVAL: Duration = Duration::from_secs(10);
// Max Buffer size
const BATCH_SIZE: usize = 100;

#[derive(Debug)]
struct SendError(String);

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SendError: {}", self.0)
    }
}

impl Error for SendError {}

pub async fn run_server(addr: String, destination_addr: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    // initialise the logger
    INIT.call_once(||{Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();});

    info!("Starting TCP log server on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let (flush_sender, flush_receiver) = mpsc::channel(100);

    let flusher_handle = tokio::spawn(buffer_flusher(Arc::clone(&buffer), flush_receiver, destination_addr));

    let timer_handle = start_flush_timer(flush_sender.clone());

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((socket, addr)) => {
                        info!("New client connected: {}", addr);
                        let buffer = Arc::clone(&buffer);
                        let flush_sender = flush_sender.clone();
                        tokio::spawn(handle_client(socket, addr, buffer, flush_sender));
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
                break;
            }
        }
    }

    info!("Shutting down server");
    drop(flush_sender);
    timer_handle.abort();
    
    if let Err(e) = flusher_handle.await {
        error!("Error waiting for flusher to complete: {}", e);
    }

    Ok(())
}

fn start_flush_timer(flush_sender: mpsc::Sender<()>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = interval(FLUSH_INTERVAL);
        loop {
            interval.tick().await;
            debug!("Timer tick, sending flush signal");
            if flush_sender.send(()).await.is_err() {
                error!("Failed to send flush signal");
                break;
            }
        }
        debug!("Flush timer stopped");
    })
}

async fn buffer_flusher(buffer: Arc<Mutex<Vec<String>>>, mut flush_receiver: mpsc::Receiver<()>, destination_addr: String) {
    while flush_receiver.recv().await.is_some() {
        let mut buffer = buffer.lock().await;
        if !buffer.is_empty() {
            debug!("Flushing buffer with {} messages", buffer.len());
            if let Err(e) = send_batch_to_destination(&buffer, &destination_addr).await {
                error!("Failed to send batch to destination: {}", e);
            } else {
                info!("Successfully flushed buffer with {} messages", buffer.len());
                buffer.clear();
            }
        } else {
            debug!("Flush signal received, but buffer is empty");
        }
    }
    debug!("Flusher shutting down");
}

async fn handle_client(mut socket: TcpStream, addr: std::net::SocketAddr, buffer: Arc<Mutex<Vec<String>>>, flush_sender: mpsc::Sender<()>) {
    let mut message = String::new();
    loop {
        let mut buf = [0; 1024];
        match socket.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                message.push_str(&String::from_utf8_lossy(&buf[..n]));
                process_messages(&mut message, &buffer, &flush_sender).await;
            }
            Err(e) => {
                warn!("Failed to read from socket: {}", e);
                break;
            }
        }
    }
    info!("Client disconnected: {}", addr);
}

async fn process_messages(message: &mut String, buffer: &Arc<Mutex<Vec<String>>>, flush_sender: &mpsc::Sender<()>) {
    while let Some(index) = message.find('\n') {
        let complete_message = message[..index].to_string();
        *message = message[index + 1..].to_string();

        let mut buffer = buffer.lock().await;
        buffer.push(complete_message.clone());
        debug!("Received message: {}", complete_message);

        if buffer.len() >= BATCH_SIZE {
            debug!("Buffer full, sending flush signal");
            drop(buffer);
            if flush_sender.send(()).await.is_err() {
                error!("Failed to send flush signal");
                return;
            }
        }
    }
}

async fn send_batch_to_destination(batch: &[String], destination_addr: &str) -> Result<(), SendError> {
    debug!("Attempting to connect to destination: {}", destination_addr);
    let mut stream = TcpStream::connect(destination_addr).await
        .map_err(|e| SendError(format!("Connection failed: {}", e)))?;

    for (i, message) in batch.iter().enumerate() {
        stream.write_all(message.as_bytes()).await
            .map_err(|e| SendError(format!("Failed to write message {}: {}", i, e)))?;
        stream.write_all(b"\n").await
            .map_err(|e| SendError(format!("Failed to write newline after message {}: {}", i, e)))?;
        debug!("Sent message {} to destination", i + 1);
    }

    stream.flush().await
        .map_err(|e| SendError(format!("Failed to flush stream: {}", e)))?;
    
    debug!("Successfully sent batch of {} messages to destination", batch.len());
    Ok(())
}