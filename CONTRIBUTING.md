# Contributing

## Development Setup

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

## Testing

Unit tests (no engine required):
```bash
cargo test
```

Integration tests (requires running engine via Docker):
```bash
cargo test --features integration
```

## Conventions

- TDD: write failing tests first
- All tests must pass before PR
- No `clippy` warnings
