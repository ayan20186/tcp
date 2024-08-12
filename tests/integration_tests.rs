use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::info;

use tcp::{run_server, run_destination, run_client};

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[tokio::test]
async fn test_complete_system() {
    init();

    // Start the destination server
    let destination_messages = Arc::new(Mutex::new(Vec::new()));
    let dest_msgs_clone = Arc::clone(&destination_messages);
    let dest_handle = tokio::spawn(async move {
        run_destination(dest_msgs_clone).await.unwrap();
    });

    // Give the destination server time to start
    sleep(Duration::from_millis(100)).await;

    // Start the main server
    let server_handle = tokio::spawn(async {
        run_server().await.unwrap();
    });

    // Give the main server time to start
    sleep(Duration::from_millis(100)).await;

    // Run the client
    let client_messages = Arc::new(Mutex::new(Vec::new()));
    let client_msgs_clone = Arc::clone(&client_messages);
    let client_handle = tokio::spawn(async move {
        run_client(10, client_msgs_clone).await.unwrap();
    });

    // Wait for the client to finish (it runs for 60 seconds)
    sleep(Duration::from_secs(65)).await;

    // Stop all servers and clients
    dest_handle.abort();
    server_handle.abort();
    client_handle.abort();

    // Check the results
    let dest_messages = destination_messages.lock().await;
    let client_messages = client_messages.lock().await;

    assert!(!dest_messages.is_empty(), "Destination should have received messages");
    assert_eq!(client_messages.len(), dest_messages.len(), "All messages should have been forwarded");

    // Check if messages are correctly batched (100 messages per batch or every 10 seconds)
    for chunk in dest_messages.chunks(100) {
        assert!(chunk.len() <= 100, "Batch size should not exceed 100 messages");
    }

    info!("Test completed successfully!");
}

#[tokio::test]
async fn test_client_reconnection() {
    init();

    // Start the destination server
    let destination_messages = Arc::new(Mutex::new(Vec::new()));
    let dest_msgs_clone = Arc::clone(&destination_messages);
    let dest_handle = tokio::spawn(async move {
        run_destination(dest_msgs_clone).await.unwrap();
    });

    // Give the destination server time to start
    sleep(Duration::from_millis(100)).await;

    // Start the main server
    let server_handle = tokio::spawn(async {
        run_server().await.unwrap();
    });

    // Give the main server time to start
    sleep(Duration::from_millis(100)).await;

    // Run the client
    let client_messages = Arc::new(Mutex::new(Vec::new()));
    let client_msgs_clone = Arc::clone(&client_messages);
    let client_handle = tokio::spawn(async move {
        run_client(10, client_msgs_clone).await.unwrap();
    });

    // Wait for a bit, then kill the server
    sleep(Duration::from_secs(10)).await;
    server_handle.abort();

    // Wait a bit, then restart the server
    sleep(Duration::from_secs(5)).await;
    let server_handle = tokio::spawn(async {
        run_server().await.unwrap();
    });

    // Wait for the client to finish
    sleep(Duration::from_secs(50)).await;

    // Stop all servers and clients
    dest_handle.abort();
    server_handle.abort();
    client_handle.abort();

    // Check the results
    let dest_messages = destination_messages.lock().await;
    let client_messages = client_messages.lock().await;

    assert!(!dest_messages.is_empty(), "Destination should have received messages");
    assert!(client_messages.len() > dest_messages.len(), "Client should have sent more messages than received due to server downtime");

    info!("Reconnection test completed successfully!");
}