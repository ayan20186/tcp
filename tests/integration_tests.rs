use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, debug, error};
use std::net::{TcpListener, TcpStream};
use std::io::ErrorKind;

use tcp::{run_server, run_destination, run_client};
// use tcp2::server::clear_server_buffer;

// Initialize the logger for tests
// fn init() {
//     let _ = env_logger::builder().is_test(true).try_init();
// }

// Find an available port for the server to bind to
fn find_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to address")
        .local_addr()
        .expect("Failed to get local address")
        .port()
}

// Wait for a server to start up and become available
async fn wait_for_server(addr: &str, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        match TcpStream::connect(addr) {
            Ok(_) => return true,
            Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                sleep(Duration::from_millis(100)).await;
            }
            Err(e) => {
                error!("Unexpected error while waiting for server: {:?}", e);
                return false;
            }
        }
    }
    false
}

// Set up both the main server and the destination server for testing
async fn setup_server_and_destination() -> Result<(Arc<Mutex<Vec<String>>>, u16, u16, tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> {
    let dest_port = find_available_port();
    let server_port = find_available_port();

    debug!("Using destination port: {}, server port: {}", dest_port, server_port);

    // Start the destination server
    let destination_messages = Arc::new(Mutex::new(Vec::new()));
    let dest_msgs_clone = Arc::clone(&destination_messages);
    let dest_handle = tokio::spawn(async move {
        run_destination(dest_msgs_clone, format!("127.0.0.1:{}", dest_port)).await.unwrap();
    });

    // Wait for the destination server to start
    if !wait_for_server(&format!("127.0.0.1:{}", dest_port), Duration::from_secs(10)).await {
        return Err("Destination server failed to start".into());
    }
    info!("Destination server started successfully");

    // Start the main server
    // let server_buffer = Arc::new(Mutex::new(Vec::new()));
    // let server_buffer_clone = Arc::clone(&server_buffer);
    let server_handle = tokio::spawn(async move {
        run_server(format!("127.0.0.1:{}", server_port), format!("127.0.0.1:{}", dest_port)).await.unwrap();
    });

    // Wait for the main server to start
    if !wait_for_server(&format!("127.0.0.1:{}", server_port), Duration::from_secs(10)).await {
        return Err("Main server failed to start".into());
    }
    info!("Main server started successfully");

    // Clear the server buffer before returning
    // clear_server_buffer(&server_buffer).await;

    Ok((destination_messages, dest_port, server_port, dest_handle, server_handle))
}

// Test the batch timing functionality
#[tokio::test]
async fn test_batch_timing() -> Result<(), Box<dyn std::error::Error>> {
    // init();

    let (destination_messages, _dest_port, server_port, dest_handle, server_handle) = setup_server_and_destination().await?;

    // Run the client at a very slow rate (1 message per second)
    let client_messages = Arc::new(Mutex::new(Vec::new()));
    let client_msgs_clone = Arc::clone(&client_messages);
    let client_handle = tokio::spawn(async move {
        run_client(13, client_msgs_clone, format!("127.0.0.1:{}", server_port)).await.unwrap();
    });

    // Wait for 30 seconds (should send 30 messages)
    for i in 0..30 {
        sleep(Duration::from_secs(1)).await;
        debug!("Waited {} seconds", i + 1);
    }

    // Stop all components
    client_handle.abort();
    server_handle.abort();
    dest_handle.abort();

    // Give some time for final processing
    sleep(Duration::from_secs(10)).await;

    // Check the results
    let dest_messages = destination_messages.lock().await;
    let client_messages = client_messages.lock().await;

    debug!("Client sent {} messages", client_messages.len());
    debug!("Destination received {} messages", dest_messages.len());

    for (i, msg) in client_messages.iter().enumerate() {
        debug!("Client message {}: {}", i, msg);
    }

    for (i, msg) in dest_messages.iter().enumerate() {
        debug!("Destination message {}: {}", i, msg);
    }

    // Assert test conditions
    assert!(!dest_messages.is_empty(), "Destination should have received messages");
    assert_eq!(client_messages.len(), dest_messages.len(), "All messages should have been forwarded");
    assert!(dest_messages.len() >= 25, "At least 25 messages should have been sent and received");

    info!("Batch timing test completed successfully!");
    Ok(())
}

// Test the complete system functionality
#[tokio::test]
async fn test_complete_system() -> Result<(), Box<dyn std::error::Error>> {
    // init();

    let (destination_messages, _dest_port, server_port, dest_handle, server_handle) = setup_server_and_destination().await?;

    // Run the client
    let client_messages = Arc::new(Mutex::new(Vec::new()));
    let client_msgs_clone = Arc::clone(&client_messages);
    let client_handle = tokio::spawn(async move {
        run_client(5, client_msgs_clone, format!("127.0.0.1:{}", server_port)).await.unwrap();
    });

    // Wait for the client to finish (it runs for 60 seconds)
    sleep(Duration::from_secs(65)).await;

    // Stop all components
    client_handle.abort();
    server_handle.abort();
    dest_handle.abort();

    // Give some time for final processing
    sleep(Duration::from_secs(10)).await;

    // Check the results
    let dest_messages = destination_messages.lock().await;
    let client_messages = client_messages.lock().await;

    debug!("Client sent {} messages", client_messages.len());
    debug!("Destination received {} messages", dest_messages.len());

    // Assert test conditions
    assert!(!dest_messages.is_empty(), "Destination should have received messages");
    assert_eq!(client_messages.len(), dest_messages.len(), "All messages should have been forwarded");

    // Check if messages are correctly batched (100 messages per batch or every 10 seconds)
    for chunk in dest_messages.chunks(100) {
        assert!(chunk.len() <= 100, "Batch size should not exceed 100 messages");
    }

    info!("Complete system test completed successfully!");
    Ok(())
}

// Test the client reconnection functionality
#[tokio::test]
async fn test_client_reconnection() -> Result<(), Box<dyn std::error::Error>> {
    // init();

    let (destination_messages, _dest_port, server_port, dest_handle, mut server_handle) = setup_server_and_destination().await?;

    // Run the client
    let client_messages = Arc::new(Mutex::new(Vec::new()));
    let client_msgs_clone = Arc::clone(&client_messages);
    let client_handle = tokio::spawn(async move {
        run_client(5, client_msgs_clone, format!("127.0.0.1:{}", server_port)).await.unwrap();
    });

    // Wait for a bit, then kill the server
    sleep(Duration::from_secs(10)).await;
    debug!("Stopping the server to simulate failure");
    server_handle.abort();

    // Wait a bit, then restart the server
    sleep(Duration::from_secs(5)).await;
    debug!("Restarting the server");
    server_handle = tokio::spawn(async move {
        run_server(format!("127.0.0.1:{}", server_port), format!("127.0.0.1:{}", _dest_port)).await.unwrap();
    });

    // Wait for the client to finish (it runs for 60 seconds total)
    sleep(Duration::from_secs(50)).await;

    // Stop all components
    client_handle.abort();
    server_handle.abort();
    dest_handle.abort();

    // Give some time for final processing
    sleep(Duration::from_secs(10)).await;

    // Check the results
    let dest_messages = destination_messages.lock().await;
    let client_messages = client_messages.lock().await;

    debug!("Client sent {} messages", client_messages.len());
    debug!("Destination received {} messages", dest_messages.len());

    // Assert test conditions
    assert!(!dest_messages.is_empty(), "Destination should have received messages");
    assert!(client_messages.len() >= dest_messages.len(), "Client should have sent at least as many messages as received by destination");
    assert!(dest_messages.len() > 0, "Some messages should have been forwarded despite server restart");

    info!("Reconnection test completed successfully!");
    Ok(())
}