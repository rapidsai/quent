# syntax=docker/dockerfile:1

FROM ghcr.io/prefix-dev/pixi:0.72.2 AS builder

WORKDIR /quent

COPY . .

RUN --mount=type=cache,target=/root/.cache/rattler/cache \
    --mount=type=cache,target=/root/.cargo/registry \
    pixi run --frozen pnpm --dir ui install --frozen-lockfile --reporter=append-only && \
    pixi run --frozen cargo build --release --locked \
        -p quent-simulator-server \
        -p quent-simulator \
        --features quent-simulator-server/ui && \
    cp target/release/quent-simulator-server target/release/quent-simulator /quent/

FROM debian:trixie AS runtime

WORKDIR /quent

RUN groupadd --system quent && \
    useradd --system --gid quent --home-dir /quent --no-create-home quent && \
    install -d -o quent -g quent /quent/events

COPY --from=builder /quent/quent-simulator-server /quent/quent-simulator-server
COPY --from=builder /quent/quent-simulator /quent/quent-simulator

EXPOSE 8080
EXPOSE 7836

USER quent
