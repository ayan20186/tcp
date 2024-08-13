use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::time::{interval_at, Instant as TokioInstant, Duration};
use log::{info, error};
use std::{env, io};
use std::time::Instant;
use rand::Rng;
use chrono;
use std::sync::Arc;
use env_logger::Builder;
use chrono::Local;
use log::LevelFilter;
use std::io::Write;
use tokio::sync::Mutex;

// Delay between reconnection attempts
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub async fn run_client(messages_per_second: u64, sent_messages: Arc<Mutex<Vec<String>>>, server_addr: String) -> io::Result<()> {

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
    // Calculate the interval between messages based on the desired messages per second
    let interval = Duration::from_micros(1_000_000 / messages_per_second);

    // Log the start of the client
    info!("Starting TCP log client, sending {} messages per second to {}", messages_per_second, server_addr);

    // Continuously attempt to connect and send messages
    loop {
        match connect_and_send(interval, Arc::clone(&sent_messages), &server_addr).await {
            Ok(_) => break,
            Err(e) => {
                error!("Error: {}. Reconnecting in {} seconds...", e, RECONNECT_DELAY.as_secs());
                tokio::time::sleep(RECONNECT_DELAY).await;
            }
        }
    }

    Ok(())
}

async fn connect_and_send(interval: Duration, sent_messages: Arc<Mutex<Vec<String>>>, server_addr: &str) -> io::Result<()> {
    // Establish a TCP connection to the server
    let mut stream = TcpStream::connect(server_addr).await?;
    info!("Connected to server at {}", server_addr);

    // Set up timing variables for message sending
    let start = TokioInstant::now();
    let mut interval_timer = interval_at(start, interval);
    let start_time = Instant::now();
    let mut message_count = 0;

    // Main message sending loop
    loop {
        // Wait for the next interval before sending the message
        interval_timer.tick().await;

        // Generate and send a log message
        let log_message = generate_log_message();
        stream.write_all(log_message.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        // Store the sent message
        sent_messages.lock().await.push(log_message.clone());

        message_count += 1;

        // Log progress every 100 messages
        if message_count % 100 == 0 {
            info!("Sent {} messages", message_count);
        }

        // Check if 60 seconds have elapsed (benchmark duration)
        if start_time.elapsed() >= Duration::from_secs(60) {
            info!("Benchmark complete. Sent {} messages in 60 seconds", message_count);
            break;
        }
    }

    Ok(())
}

fn generate_log_message() -> String {
    // Define possible log levels
    let log_levels = ["INFO", "WARN", "ERROR", "DEBUG"];
    let mut rng = rand::thread_rng();
    // Randomly select a log level
    let level = log_levels[rng.gen_range(0..log_levels.len())];
    // Generate a timestamp for the log message
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    // Create a sample log message with a random number
    let message = format!("Sample log message number {}", rng.gen_range(1..10000));

    // Combine all parts into a formatted log message
    format!("[{}] {} - {}", timestamp, level, message)
}


#[tokio::main]
async fn main() -> io::Result<()> {

    // takes command line input for number of message per second
    let args: Vec<String> = env::args().collect();
    let messages_per_second: u64 = match args.get(1) {
        Some(arg) => arg.parse().expect("Please provide a valid number for messages per second"),
        None => {
            eprintln!("Usage: {} <messages_per_second>", args[0]);
            std::process::exit(1);
        }
    };
    // Create a shared vector to store sent messages
    let sent_messages = Arc::new(Mutex::new(Vec::new()));
    // Run the client with 10 messages per second, connecting to localhost:8080
    run_client(messages_per_second, sent_messages, "127.0.0.1:8080".to_string()).await
}