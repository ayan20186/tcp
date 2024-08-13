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
use std::sync::Once;


static INIT: Once = Once::new();


// delay b/w reconnections
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub async fn run_client(messages_per_second: u64, sent_messages: Arc<Mutex<Vec<String>>>, server_addr: String) -> io::Result<()> {

    
    // calculate the interval between messages
    // based on the desired messages per second, so that there is no message loss
    let interval = Duration::from_micros(1_000_000 / messages_per_second);

    info!("Starting TCP log client, sending {} messages per second to {}", messages_per_second, server_addr);

    // connect and send messages, until it works
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
    // establish a TCP connection to the server
    let mut stream = TcpStream::connect(server_addr).await?;
    info!("Connected to server at {}", server_addr);

    // set up timing variables for message sending
    let start = TokioInstant::now();
    let mut interval_timer = interval_at(start, interval);
    let start_time = Instant::now();
    let mut message_count = 0;
    let mut last_log_time = Instant::now();

    // loop for sending messages
    loop {
        // waiting for the next interval 
        // before sending mssgs
        interval_timer.tick().await;

        // generating and sending a log message
        let log_message = generate_log_message();
        stream.write_all(log_message.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        println!("{}", log_message);

        // store the message that was sent in buffer
        sent_messages.lock().await.push(log_message.clone());

        message_count += 1;

        // logging every 10 seconds
        if last_log_time.elapsed() >= Duration::from_secs(10) {
            info!("Sent {} messages", message_count);
            last_log_time = Instant::now();
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
    // defining possible logs
    let log_levels = ["INFO", "WARN", "ERROR", "DEBUG"];
    let mut rng = rand::thread_rng();
    // select a log message
    let level = log_levels[rng.gen_range(0..log_levels.len())];
    // generate timestamp for the log message
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    // creating a sample log message
    let message = format!("Sample log message number {}", rng.gen_range(1..10000));

    // return the log message
    format!("[{}] {} - {}", timestamp, level, message)
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> io::Result<()> {

    // Initialize the logger
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
    // Run the client with messages per second, connecting to localhost:8080
    run_client(messages_per_second, sent_messages, "127.0.0.1:8080".to_string()).await
}