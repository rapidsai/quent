# syntax=docker/dockerfile:1

FROM rust:1.91-trixie AS builder

WORKDIR /quent

# Build deps
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler

# Copy source
COPY . .

# Build server executables
RUN cargo build --release -p quent-server
RUN cargo build --release --example simulator

# Support running both server and simulator executables.
FROM debian:trixie AS runtime

WORKDIR /quent

COPY --from=builder /quent/target/release/quent-server /quent/quent-server
COPY --from=builder /quent/target/release/examples/simulator /quent/simulator

# Expose default analyzer (HTTP) and collector (gRPC) ports
EXPOSE 8080
EXPOSE 7836

ENTRYPOINT ["/bin/sh","-c"]
