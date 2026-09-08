# Development guide

## Development environment

Install [Pixi](https://pixi.sh), then enter the repository environment:

```bash
pixi shell
```

Pixi is the canonical development environment and provides the required Rust,
Node.js, pnpm, and protoc versions. Use another approach at your own risk.

## Repository checks

Run the Rust checks from the repository root:

```bash
pixi run cargo fmt --all -- --check
pixi run cargo clippy --all-targets --all-features --locked -- -D warnings
pixi run cargo test --all-features --locked --all-targets
```

These commands check the workspace's default members on every supported
platform. Full `--workspace` checks also include opt-in NVTX and
language-bridge crates and are intended for Linux.

Check Markdown with the version used by CI:

```bash
pixi run --frozen uvx rumdl==0.1.67 check --diff
```

Run the UI checks from `ui/`:

```bash
pnpm install
pnpm ci:check
```

## Run the query-engine development stack

This workflow additionally requires Docker with the Compose plugin; Pixi does
not install Docker.

Docker Compose provides a simulator server and sample event data:

```bash
docker compose up --build
```

The collector listens on port `7836`, and the analysis API listens on port
`8080`. This development image intentionally omits the embedded UI so the
frontend can run separately with Vite and hot reload.

## Run the UI development server

Start Vite separately:

```bash
cd ui
pnpm install
pnpm dev
```

The UI is available at <http://localhost:5173>. `pnpm dev` generates the
TypeScript bindings before starting Vite. Run `pnpm bindings` after changing
Rust types while the development server remains open.

## Run without Docker

Start the simulator server with CORS enabled for Vite:

```bash
cargo run -p quent-simulator-server -- --cors-address http://localhost:5173
```

Generate a test dataset from another shell:

```bash
cargo run -p quent-simulator -- --exporter collector
```

## Build with the bundled UI

The `ui` feature builds and embeds the static webpage in the server:

```bash
cargo build -p quent-simulator-server --features ui --release
```

## Enable Swagger UI

Build the server with the `swagger` feature:

```bash
cargo build -p quent-simulator-server --features ui,swagger --release
```

Then visit <http://localhost:8080/swagger-ui>.

## Related documentation

- [Contribution guide](CONTRIBUTING.md)
- [UI development guide](ui/README.md)
