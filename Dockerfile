
FROM rust:1.78 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd -m atlas && mkdir -p /data && chown atlas:atlas /data
COPY --from=builder /app/target/release/atlas4d-uni /usr/local/bin/atlas4d-uni
USER atlas
ENV ATLAS4D_DATA_DIR=/data
EXPOSE 50051
ENTRYPOINT ["atlas4d-uni","--data-dir","/data","serve","--addr","0.0.0.0:50051"]
