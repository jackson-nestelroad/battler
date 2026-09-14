# Battler Web App

This is the React + TypeScript + Vite frontend web application for playing and observing battles.

---

## Development Guide

### 1. Build WASM Core Modules
The frontend web application resolves the WASM state machine directly from the workspace. Ensure you compile the WASM targets first:

```bash
# Build the WASM state package for Node/tests
npm run build:battler-state

# Build the WASM state package for Browser/Vite bundler (specifically mapped in vite.config.ts)
npx wasm-pack build --target bundler --out-dir pkg/bundler battler-state -- --features typescript

# Generate TS Bindings
npm run build:bindings
```

### 2. Build JS Clients
Compile the JS packages representing the WAMP communication layer:

```bash
npm run build --workspace=battler-client
npm run build --workspace=battler-service-client
npm run build --workspace=battler-web-app
```

---

## Running the Servers

### 1. Start the Rust WAMP Server
The server manages battles, accepts choice submissions, and publishes pub/sub streams. Run it with:

```bash
cargo run -p battler-server -- \
  --port 8080 \
  --data-dir battle-data/data \
  --realm-name battler \
  --realm-uri com.battler
```

### 2. Start the Frontend React Web App
Run the Vite development server to launch the frontend:

```bash
# Deletes optimized cache and forces Vite to pick up local changes
rm -rf js-clients/battler-web-app/node_modules/.vite
npm run dev --workspace=battler-web-app -- --force
```

Open [http://localhost:5173/](http://localhost:5173/) in your browser (force refresh with `Cmd+Shift+R` to clear cached browser WASM assets).

---

## Testing Battles

1. Connect to the server from the frontend (`ws://127.0.0.1:8080/`, realm: `battler`).
2. Navigate to the **Matchmaking Lobby**.
3. Create side-by-side tabs, log in as `@gary` and `@ash`, propose a battle, accept, select teams, and start fighting!
