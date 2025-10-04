# USSD Client - HTTP Bridge

## Overview

The `ussdclient` is an HTTP bridge service that connects USSD gateway providers (like Africa's Talking) to the SpacetimeDB backend. It translates HTTP requests from USSD sessions into SpacetimeDB reducer calls and manages the bidirectional communication flow.

## Architecture

This is a **Rust HTTP server** built with Axum that provides:
- HTTP endpoints for USSD gateway webhooks
- Integration with SpacetimeDB client SDK
- Request/response transformation
- Session state management bridge

## Key Features

### HTTP Server
- **RESTful API** - Handles incoming USSD requests from gateway providers
- **Webhook Support** - Processes real-time USSD interactions
- **Request Validation** - Ensures proper USSD request format
- **Response Formatting** - Converts SpacetimeDB responses to USSD format

### SpacetimeDB Integration
- **Client SDK** - Connects to SpacetimeDB module
- **Reducer Calls** - Invokes ussdgeth reducers based on USSD input
- **Real-time Updates** - Subscribes to database changes
- **Error Handling** - Manages connection failures and retries

### USSD Protocol
- **Session Management** - Tracks USSD session lifecycle
- **Menu Navigation** - Processes user menu selections
- **Input Validation** - Validates user inputs (amounts, addresses, etc.)
- **Response Generation** - Creates appropriate USSD response messages

## Development

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://rustup.rs | sh

# Start SpacetimeDB (required dependency)
spacetime start
```

### Building
```bash
# Build the HTTP server
cargo build --release

# Run in development mode
cargo run
```

### Testing
```bash
# Run unit tests
cargo test

# Test HTTP endpoints
curl -X POST http://localhost:3000/ussd -d '{"sessionId":"123","phoneNumber":"+254712345678","text":"*384*1*2#"}'
```

## Configuration

### Environment Variables
Create a `.env` file in the ussdclient directory:

```env
# SpacetimeDB connection
SPACETIMEDB_URL=http://localhost:3000
SPACETIMEDB_MODULE=ussdgeth

# HTTP server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# USSD Gateway (Africa's Talking)
AT_API_KEY=your_api_key_here
AT_USERNAME=your_username_here
AT_SHORTCODE=*384*1#
```

## Usage

### Starting the Server
```bash
# Development
cargo run

# Production
cargo run --release
```

### API Endpoints

#### POST /ussd
Handles incoming USSD requests from the gateway.

**Request Body:**
```json
{
  "sessionId": "ATUid_session_id",
  "phoneNumber": "+254712345678",
  "networkCode": "63902",
  "serviceCode": "*384*1#",
  "text": "1*2*1000000000000000000"
}
```

**Response:**
```json
{
  "text": "CON Enter recipient address:\n",
  "continue": true
}
```

#### GET /health
Health check endpoint for load balancers.

**Response:**
```json
{
  "status": "healthy",
  "spacetimedb": "connected"
}
```

## File Structure

```
ussdclient/
├── src/
│   ├── main.rs            # HTTP server setup and routing
│   ├── handlers/          # HTTP request handlers
│   ├── spacetime/         # SpacetimeDB client integration
│   └── ussd/              # USSD protocol handling
├── Cargo.toml             # Dependencies and build config
├── .env.example           # Environment variables template
└── README.md              # This file
```

## Dependencies

- `axum` - Modern HTTP server framework
- `tokio` - Async runtime
- `reqwest` - HTTP client for external APIs
- `serde` - JSON serialization/deserialization
- `dotenv` - Environment variable management
- `anyhow` - Error handling
- `hyper` - HTTP types and utilities

## USSD Flow

1. **User Dials** `*384*1#` on their phone
2. **Gateway** sends HTTP request to `/ussd` endpoint
3. **ussdclient** processes request and calls appropriate SpacetimeDB reducer
4. **SpacetimeDB** updates state and returns response
5. **ussdclient** formats response for USSD protocol
6. **Gateway** displays menu/message to user's phone

## Security Considerations

- **Input Validation** - All USSD inputs are sanitized
- **Rate Limiting** - Prevents abuse of HTTP endpoints
- **Authentication** - Gateway webhook verification
- **HTTPS** - TLS encryption for production deployment
- **Error Masking** - User-friendly error messages without sensitive details

## Deployment

### Docker
```dockerfile
FROM rust:alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/ussdclient /usr/local/bin/
EXPOSE 8080
CMD ["ussdclient"]
```

### Systemd Service
```ini
[Unit]
Description=USSD Client HTTP Bridge
After=network.target

[Service]
Type=simple
User=ussd
WorkingDirectory=/opt/ussdclient
ExecStart=/opt/ussdclient/target/release/ussdclient
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## Related Components

- **ussdgeth** - SpacetimeDB module for business logic
- **ethclient** - Ethereum blockchain interaction
- **contracts** - Smart contracts for account abstraction