# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Docker Compose provider plugin for Kubernetes written in Rust. It allows Docker Compose to manage services that are actually Kubernetes resources via port-forwarding. The plugin acts as a bridge between Docker Compose's service model and Kubernetes resources.

## Build and Development Commands

### Building
```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo build --locked --release # Release build with locked dependencies (used in CI)
```

### Running
The binary is invoked by Docker Compose, not directly. It follows the Docker Compose provider protocol.

Example usage (via docker-compose.yml):
```yaml
services:
  test:
    provider:
      type: docker-compose-k8s-provider
      options:
        resource: pod/traefik
        port-mapping: 3001:80
```

### Testing
Run individual services manually for testing:
```bash
# Up command (start port-forward daemon)
cargo run -- compose --project-name myproject up --resource pod/traefik --port-mapping 3001:80 test

# Down command (stop daemon)
cargo run -- compose --project-name myproject down test
```

### Cross-compilation
The project uses GitHub Actions for cross-platform builds targeting:
- macOS x86_64 (x86_64-apple-darwin)
- macOS ARM (aarch64-apple-darwin)
- Linux x86_64 (x86_64-unknown-linux-gnu)

CI runs on pushes to `releases` branch and tags matching `v.*`

## Architecture

### High-Level Flow

1. **Up Command**: Docker Compose invokes the provider with `up` command
   - Provider spawns a daemon process (double-fork + setsid)
   - Daemon creates a Unix socket at `/tmp/{project}/{service}.sock`
   - Daemon establishes port-forwarding to the specified Kubernetes resource
   - Parent process performs health checks via Unix socket before exiting

2. **Down Command**: Docker Compose invokes the provider with `down` command
   - Connects to Unix socket and sends stop request
   - Daemon terminates, cleaning up port-forwarding and socket

### Process Model

The `up` command uses a **double-fork daemon pattern** (src/cmd/up.rs):
- First fork: Creates new session with `setsid()`
- Second fork: Creates actual daemon process
- Parent waits for intermediate process, then performs health checks

This ensures the daemon is fully detached from the Docker Compose process tree.

### Communication Protocol

Binary protocol over Unix domain sockets (src/protocol.rs):
- Request types: Health, Stop
- Response types: Ok, Err
- Used for health checks and graceful shutdown

### Key Components

**src/cmd/up.rs**:
- `spawn_daemon()`: Double-fork process creation
- `run_daemon()`: Main daemon loop with two threads:
  - Health request handler (Unix socket server)
  - Port-forward manager (auto-restart on failure)
- `health_checks()`: Validates daemon startup via socket connection

**src/cmd/down.rs**:
- Connects to daemon socket and sends stop request
- Waits for socket disconnect to confirm termination

**src/net.rs**:
- `Server`: Unix socket server with non-blocking listener
- `Client`: Unix socket client with retry logic (15 attempts, 500ms intervals)
- `socket_fn()`: Generates socket path `/tmp/{project}/{service}.sock`

**src/docker.rs**:
- Macros for Docker Compose JSON-formatted logging
- Messages serialized as `{"type": "info"|"error", "message": "..."}`

**src/cli.rs**:
- Clap-based CLI parsing following Docker Compose provider interface
- Commands: `compose --project-name <name> up|down --resource <k8s-resource> --port-mapping <local:remote> <service>`

### Platform-Specific Code

**Daemonization** (src/cmd/up.rs:85-97):
- Linux: Uses `unistd::daemon()` from nix crate
- macOS: Uses deprecated `libc::daemon()` via unsafe FFI

## Important Implementation Details

### Socket Management
Sockets are created in `/tmp/{project}/{service}.sock`. The server checks for existing listeners before binding to prevent conflicts (src/net.rs:19-21).

### Port-Forward Process Management
The daemon continuously monitors and restarts the port-forward process if it exits (src/cmd/up.rs:144-161). This ensures resilience against network disruptions.

### Health Check Timeouts
- Client connection attempts: 7.5 second timeout (15 retries × 500ms)
- Daemon disconnect waits: 1 second timeout (10 retries × 100ms)

### Fork Safety
The code uses unsafe `fork()` but is safe because the program is single-threaded before forking (src/cmd/up.rs:173-180).
