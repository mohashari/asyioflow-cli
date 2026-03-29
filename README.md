# asyioflow-cli

[![CI](https://github.com/mohashari/asyioflow-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/mohashari/asyioflow-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/asyioflow-cli)](https://crates.io/crates/asyioflow-cli)

CLI for the [AysioFlow](https://github.com/mohashari/asyioflow-engine) distributed workflow engine.

## Install

```bash
cargo install asyioflow-cli
```

## Quick Start

```bash
# Submit a job
asyioflow job submit --type email --payload '{"to":"user@example.com"}'

# List running jobs
asyioflow job list --status running

# Get job details
asyioflow job get <job-id>

# Cancel a job
asyioflow job cancel <job-id>

# Run a workflow
asyioflow workflow run pipeline.yaml

# Check engine health
asyioflow status

# View metrics
asyioflow metrics
```

## Configuration

| Setting | Flag | Env var | Default |
|---------|------|---------|---------|
| gRPC address | `--grpc-addr` | `ASYIOFLOW_GRPC_ADDR` | `localhost:9090` |
| REST address | `--rest-addr` | `ASYIOFLOW_REST_ADDR` | `http://localhost:8080` |

## Workflow File Format

```yaml
name: my-pipeline
steps:
  - name: fetch
    job_type: http-fetch
    payload: {}
    depends_on: []
  - name: transform
    job_type: data-transform
    payload: { key: value }
    depends_on: [fetch]
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Logical failure (job not found, workflow step failed) |
| 2 | Engine unreachable |
| 3 | Bad CLI arguments |
