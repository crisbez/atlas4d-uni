
# Atlas4D Unique v0.2.1 – gRPC + Binary Segments + Docker + CI

## Docker (една команда)
```bash
docker compose up --build -d
# gRPC слуша на :50051, данните са в ./atlas4d-data
```

## gRPC Proto
`proto/atlas4d.proto` (tonic/prost)

### grpcurl примери
```bash
grpcurl -d '{
  "observations": [{
    "entity_id":"f32f3b86-3f5e-4a75-9a2d-9a6d6f2f7c2a",
    "t":"2025-08-19T04:05:10Z",
    "pos":{"lat":42.5001,"lon":27.4802,"alt_m":11.2},
    "quality":0.95,
    "sigma_m":1.5,
    "source_json":"{\"sensor\":\"demo\"}"
  }]
}' -plaintext localhost:50051 atlas4d.Atlas4D/IngestMany

grpcurl -d '{
  "lat":42.50,"lon":27.48,"radius_m":50,
  "t0":"2025-08-19T04:00:00Z","t1":"2025-08-19T05:00:00Z",
  "limit":100
}' -plaintext localhost:50051 atlas4d.Atlas4D/QueryNear

grpcurl -d '{}' -plaintext localhost:50051 atlas4d.Atlas4D/ListSegments
```

## CI за образ (Docker Hub / GHCR)
- Файл: `.github/workflows/docker.yml`
- Пусни **release tag** например `v0.2.1` → ще се билднат multi-arch образи:
  - `ghcr.io/<owner>/atlas4d-uni:latest` и `:v0.2.1`
  - (по избор) `<dockerhub-username>/atlas4d-uni:latest` и `:v0.2.1`

## Локален CLI
```bash
cargo build --release
./target/release/atlas4d-uni --data-dir ./atlas4d-data demo-ingest --minutes 10
./target/release/atlas4d-uni --data-dir ./atlas4d-data compact
./target/release/atlas4d-uni --data-dir ./atlas4d-data serve --addr 0.0.0.0:50051
```
