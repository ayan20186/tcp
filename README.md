# TCP Log System

This project implements a complete TCP-based logging system using the Tokio async runtime in Rust. It consists of three main components: a TCP server that receives and buffers log messages, a client that generates and sends log messages, and a destination server that receives the batched messages.

Drive Link: 
Rust ass2 Part 1: https://drive.google.com/file/d/1nWn6hwfNrhHI-IksNMFfHomf0VMYnQSe/view?usp=sharing


## Features

- Asynchronous TCP server, client, and destination server built with Tokio
- Client generates and sends log messages at a configurable rate
- Server receives log messages from multiple clients, buffers them, and forwards them in batches
- Destination server receives and stores batched messages
- Graceful shutdown on CTRL+C for all components
- Configurable message rates and buffer sizes
- Robust error handling and reconnection logic

## Components

### 1. TCP Server (server.rs)

- Listens for incoming log messages on a specified port
- Buffers incoming messages (batch size: 100 messages)
- Sends batches to the destination server every 10 seconds or when the buffer is full
- Handles multiple client connections concurrently

### 2. Client Simulator (client.rs)

- Generates sample log messages at a configurable rate
- Sends log messages to the TCP server
- Handles reconnection if the connection to the server is lost
- Runs a 60-second benchmark by default

### 3. Destination Server (destination.rs)

- Receives batched messages from the TCP server
- Stores received messages in a shared vector
- Handles multiple connections from the TCP server

## Usage

### Starting the TCP Server

```bash
cargo run --bin server
```

This starts the server listening on `127.0.0.1:8080` and forwarding messages to `127.0.0.1:9090`.

### Running the Client Simulator

```bash
cargo run --bin client <messages_per_second>
```

Replace `<messages_per_second>` with the desired message rate.

### Starting the Destination Server

```bash
cargo run --bin destination
```

This starts the destination server listening on `127.0.0.1:9090`.

## Implementation Details

### Server Configuration

- **Flush Interval**: 10 seconds
- **Batch Size**: 100 messages
- **Reconnection Delay**: 5 seconds (for client)

### Client Configuration

- **User defined messages per second**:
- **Client connection duration**: 60 seconds

### Logging

All components use the `log` and `env_logger` crates for logging, with the following levels:
- INFO: General operational messages
- ERROR: Critical errors
- WARN: Warning messages
- DEBUG: Detailed debugging information

## Development

To set up the development environment:

1. Ensure you have Rust and Cargo installed
2. Clone this repository
3. Run `cargo build` to compile the project

## Project Structure

- `main.rs`: Entry point of the application
- `server.rs`: Contains the main TCP server logic
- `client.rs`: Implements the client simulator
- `destination.rs`: Implements the destination server

## Error Handling

The system implements custom error handling and logging for various operations, providing detailed error messages for debugging.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
