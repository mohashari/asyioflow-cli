# Changelog

## [0.1.0] - 2026-03-28

### Added
- `asyioflow job submit/get/cancel/list` — job management via gRPC + REST
- `asyioflow workflow run` — DAG workflow execution with TTY live progress
- `asyioflow status` — engine reachability + per-status job counts
- `asyioflow metrics` — Prometheus metrics display (table/JSON)
- Global `--grpc-addr` / `--rest-addr` flags with env var fallback
