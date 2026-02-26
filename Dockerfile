# syntax=docker/dockerfile:1

FROM rust:1.91-trixie AS builder
#test
WORKDIR /quent

# Build deps
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler

# Copy source
COPY . .

# Build simulator executables
RUN cargo build --release -p quent-simulator-server
RUN cargo build --release -p quent-simulator

# Support running both server and simulator executables.
FROM debian:trixie AS runtime

WORKDIR /quent

COPY --from=builder /quent/target/release/quent-simulator-server /quent/quent-simulator-server
COPY --from=builder /quent/target/release/quent-simulator /quent/quent-simulator

# Expose default analyzer (HTTP) and collector (gRPC) ports
EXPOSE 8080
EXPOSE 7836
