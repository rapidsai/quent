# syntax=docker/dockerfile:1

FROM rust:1.91-trixie AS builder

WORKDIR /quent

RUN apt-get update && \
    apt-get install -y --no-install-recommends protobuf-compiler

COPY . .

# Build simulator executables with cached target dir and cargo registry
RUN --mount=type=cache,target=/quent/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release -p quent-simulator-server -p quent-simulator && \
    cp target/release/quent-simulator-server target/release/quent-simulator /quent/

FROM debian:trixie AS runtime

WORKDIR /quent

COPY --from=builder /quent/quent-simulator-server /quent/quent-simulator-server
COPY --from=builder /quent/quent-simulator /quent/quent-simulator

EXPOSE 8080
EXPOSE 7836
