# Quent Deployment Template

This directory contains templates for deploying a quent-server alongside the webserver+UI.

## Contents

| File | Purpose |
|---|---|
| `Dockerfile.server.template` | Builds your Rust quent-server binary |
| `docker-compose.template.yml` | Wires the server and webserver together |

The webserver+UI image is built from `Dockerfile.webserver` in the quent repo root. The compose template references the quent repo via a git URL so Docker clones and builds it automatically.

## Quick start

1. **Copy this `deploy/` directory** into your project.

2. **Edit `Dockerfile.server.template`** — replace `your-quent-server` with your crate/binary name. Add any extra build dependencies your server needs.

3. **Edit `docker-compose.template.yml`** — update the quent repo git URL to point to the correct repository and branch.

4. **Run**:

   ```sh
   docker compose -f deploy/docker-compose.template.yml up
   ```

5. Open `http://localhost:8000`.

## Environment variables

### Webserver

| Variable | Default | Description |
|---|---|---|
| `QUENT_ANALYZER_ADDRESS` | `http://localhost:8080` | URL of the quent-server analyzer API |
| `LOG_LEVEL` | `INFO` | Python log level (`DEBUG`, `INFO`, `WARNING`, `ERROR`) |
| `STATIC_DIR` | `/app/static` | Path to built UI assets inside the container |

### Server

The server container exposes ports **8080** (analyzer API) and **7836** (collector). Adjust the `command:` in the compose file to pass CLI flags to your binary.
