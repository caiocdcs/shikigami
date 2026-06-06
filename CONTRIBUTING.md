# Contributing to Shikigami

## Prerequisites

- Rust toolchain (stable)
- SQLite
- just (task runner)

## Build & Test

```bash
just build        # cargo build
just test         # cargo test
just fmt          # cargo fmt
just lint         # cargo clippy (strict: -D warnings -D clippy::unwrap_used)
just ci           # fmt-check + lint + test + audit
```

## PR Checklist

Before submitting a pull request:

- [ ] `just fmt && just lint && just test` passes with no errors
- [ ] No `.unwrap()` or `.expect()` in production code (tests are fine)
- [ ] New features include tests
- [ ] CHANGELOG.md updated if user-facing

## Coding Standards

Enforced by clippy, but worth knowing:

- Hexagonal architecture: core has no infrastructure imports
- Domain error types (MonitorError, IntegrationError) with `?` propagation
- Services orchestrate, repos persist
- See AGENTS.md for full details

## Agent-Assisted Contributions

If code was written or modified by an AI agent, include:

- A note that the contribution is agent-assisted
- Which files were consulted and what was changed
