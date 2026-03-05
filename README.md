# Quent

QUery ENgine Telemetry - an experimental telemetry framework for query engines.

## Development

### Prerequisites

- Rust (stable, >= 1.93)
- Node.js (>= 22)
- pnpm (>= 10)
- protoc (protobuf compiler)

Or use [pixi](https://pixi.sh) to manage all dependencies:

```bash
pixi shell
```

This installs the required toolchains and drops you into a shell with
everything on `PATH`.

### UI development

The easiest way to get a working backend for UI development is with Docker
Compose:

```bash
docker compose up --build
```

This spawns the simulator server (collector on `:7836`, analyzer HTTP API on
`:8080`) and runs the simulator application, which generates a test dataset
by sending simulated query engine events to the collector.

Then start the Vite dev server:

```bash
cd ui
pnpm install
pnpm dev
```

The dev server starts on <http://localhost:5173> by default.

#### Running the server without Docker

Without the `ui` feature, the server only exposes the analysis API and does not
build and serve the static webpage, so you can use the Vite dev server as
described previously.

```bash
cargo run -p quent-simulator-server -- --cors-address http://localhost:5173
```

To generate a test dataset, run the simulator:

```bash
cargo run -p quent-simulator
```

### Building with the static webpage

With the `ui` feature flag, the server also serves the static webpage, removing
the need for a separate frontend server. This approach can be useful to do some
stress testing on the UI.

```bash
cargo build -p quent-simulator-server --features ui --release
```

This runs `pnpm install && pnpm build` in `ui/` as part of the Cargo build and
bundles the output into the binary.

### Swagger UI

An interactive API explorer is available behind the `swagger` feature flag:

```bash
cargo build -p quent-simulator-server --features ui,swagger --release
```

Then visit <http://localhost:8080/swagger-ui>.
