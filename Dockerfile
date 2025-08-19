# --- Builder stage ---
FROM rustlang/rust:nightly AS builder
WORKDIR /app

# полезни инструменти за build
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler pkg-config clang ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY . .
ENV RUST_BACKTRACE=1
RUN cargo build --release -vv

# --- Runtime stage ---
FROM debian:bookworm-slim
RUN useradd -m atlas && mkdir -p /data && chown atlas:atlas /data
COPY --from=builder /app/target/release/atlas4d-uni /usr/local/bin/atlas4d-uni
USER atlas
ENV ATLAS4D_DATA_DIR=/data
EXPOSE 50051
ENTRYPOINT ["atlas4d-uni","--data-dir","/data","serve","--addr","0.0.0.0:50051"]
