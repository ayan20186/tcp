use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::time::{interval_at, Instant as TokioInstant, Duration};
use log::{info, error, LevelFilter};
use chrono::Local;
use env_logger::Builder;
use std::io;
use std::env;
use std::io::Write;
use std::time::Instant;
use rand::Rng;
use chrono;
use std::sync::Arc;
use tokio::sync::Mutex;

// address of the server
const SERVER_ADDR: &str = "127.0.0.1:8080";
// delay after which client will try re-connecting with the server
const RECONNECT_DELAY: Duration = Duration::from_secs(5);


pub async fn run_client(messages_per_second: u64, sent_messages: Arc<Mutex<Vec<String>>>) -> io::Result<()> {
    
    // calculates interval between each 
    // message based on number of messages
    let interval = Duration::from_micros(1_000_000 / messages_per_second);

    // logging
    info!("Starting TCP log client, sending {} messages per second", messages_per_second);

    // tries to connect and send message
    loop {
        match connect_and_send(interval, Arc::clone(&sent_messages)).await {
            Ok(_) => break,
            Err(e) => {
                error!("Error: {}. Reconnecting in {} seconds...", e, RECONNECT_DELAY.as_secs());
                tokio::time::sleep(RECONNECT_DELAY).await;
            }
        }
    }

    Ok(())
}

// function to connect and send message
async fn connect_and_send(interval: Duration, sent_messages: Arc<Mutex<Vec<String>>>) -> io::Result<()> {
    
    // connects with the server
    let mut stream = TcpStream::connect(SERVER_ADDR).await?;
    info!("Connected to server at {}", SERVER_ADDR);

    let start = TokioInstant::now();
    let mut interval_timer = interval_at(start, interval);
    let start_time = Instant::now();
    let mut message_count = 0;

    loop {

        // wait time before sending next message
        interval_timer.tick().await;

        let log_message = generate_log_message();
        stream.write_all(log_message.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        // adds sent message to the buffer vector
        sent_messages.lock().await.push(log_message.clone());

        message_count += 1;

        // logs message every 100 messages
        if message_count % 100 == 0 {
            info!("Sent {} messages", message_count);
        }
        //logs for 60 seconds
        if start_time.elapsed() >= Duration::from_secs(60) {
            info!("Benchmark complete. Sent {} messages in 60 seconds", message_count);
            break;
        }
    }

    Ok(())
}

// function to generate log message
fn generate_log_message() -> String {
    let log_levels = ["INFO", "WARN", "ERROR", "DEBUG"];
    let mut rng = rand::thread_rng();
    let level = log_levels[rng.gen_range(0..log_levels.len())];
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let message = format!("Sample log message number {}", rng.gen_range(1..10000));

    format!("[{}] {} - {}", timestamp, level, message)
}

//main function
#[cfg(not(test))]
#[tokio::main]
async fn main() -> io::Result<()> {

    // initialising logging
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

    // takes command line input for number of message per second
    let args: Vec<String> = env::args().collect();
    let messages_per_second: u64 = match args.get(1) {
        Some(arg) => arg.parse().expect("Please provide a valid number for messages per second"),
        None => {
            eprintln!("Usage: {} <messages_per_second>", args[0]);
            std::process::exit(1);
        }
    };

    let sent_messages = Arc::new(Mutex::new(Vec::new()));
    run_client(messages_per_second, sent_messages).await
}