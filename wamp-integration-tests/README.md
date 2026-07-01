# WAMP Interoperability & Integration Tests

This package contains integration and compatibility tests verifying the interoperability of our Rust WAMP implementation (`battler-wamp`) against standard python-wamp/Crossbar.io routers and TypeScript/AutobahnJS clients.

## Prerequisites

- **Node.js** (v20+ recommended)
- **Rust/Cargo**
- **Python 3** (Crossbar.io is written in Python/Twisted)

## Build Instructions

Before running the Node.js test suite, you must compile the Rust compatibility binary and install the Node.js dependencies.

### 1. Build the Rust Compatibility Binary
From the root of the repository, build the compatibility test binary:
```bash
cargo build -p battler-wamp-compat-test-bin
```
This builds the target binary located at `target/debug/battler-wamp-compat-test-bin`.

### 2. Setup python-wamp/Crossbar
Crossbar is run from a local Python virtual environment (`venv`) inside the `wamp-integration-tests` folder. If not already initialized, you can set it up and install Crossbar:
```bash
cd wamp-integration-tests
python3 -m venv venv
./venv/bin/pip install crossbar
```

### 3. Install Node.js Dependencies
From the `wamp-integration-tests` folder, install npm dependencies:
```bash
npm install
```

## Running Tests

To run all integration and compatibility tests, execute:
```bash
npm test
```

## Test Suite Architecture

The tests are organized into two main files under `src/`:

### 1. Client Compatibility (`src/client-compat.test.ts`)
Tests the Rust implementation running as a **Client** against a **Crossbar.io** router instance.
- Spawns the Crossbar router locally using configurations in `.crossbar/config.json`.
- Spawns the compiled `battler-wamp-compat-test-bin` in `client` mode to run various connection, publisher, subscriber, caller, and callee scenarios.
- Checks client features such as:
  - WAMP-SCRAM client authentication against Crossbar.
  - Prefix/Wildcard Pub/Sub subscriptions.
  - Progressive Call Results, Call Cancellation, and Caller Identification.

### 2. Router Compatibility (`src/router-compat.test.ts`)
Tests the Rust implementation running as a **Router** with Node.js **AutobahnJS** clients connecting to it.
- Spawns `battler-wamp-compat-test-bin` in `router` mode.
- Connects standard AutobahnJS clients to the Rust router to verify correct broker and dealer behavior.
- Verifies router functionality including:
  - Custom WAMP-SCRAM authentication handshake.
  - Pattern matching for publishers, subscribers, callers, and callees.
  - Session lifecycle and clean shutdowns.

### 3. SCRAM Cryptographic Helper (`src/scram.ts`)
Contains the client-side WAMP-SCRAM authentication logic using PBKDF2 and Argon2id. Since AutobahnJS doesn't support Argon2id natively, this helper is used by the test client instances to compute the SCRAM proof and verifier signatures required by the Rust router.
