# Shikigami - Agent Instructions

## Project Overview

Self-hosted heartbeat/cron monitor. Single binary, SQLite-backed, notifications via ntfy/gotify/slack.

## Before Finishing

Run `just fmt && just lint && just test`. All must pass. No exceptions.

## Navigation

```
src/
  api/            HTTP handlers and DTOs
  core/           Business logic (no infrastructure imports)
    domain/       Entities, value types, errors
    ports/        Trait definitions only
    *_service.rs  Orchestrators
    *_checker.rs  Background workers
  spi/           Implementations of ports (I/O only)
  lib.rs         Composition root (pool, state, wiring)
  main.rs        Entry point, signal handling
migrations/       SQL migrations (run on startup)
tests/
  integration_tests.rs  API-level tests via axum::Router
  notification_tests.rs Unit-level tests for outbox/checker/notification services
```

## Architecture

Hexagonal: core depends only on ports. SPI implements ports. API calls services, never SPI directly.

```
API handlers -> Services (orchestrators) -> Ports (traits)
                                                |
                                        SPI adapters (I/O)
```

## Testing

```bash
just test                  # all 37 tests
just test --test integration # API-level tests
just test --test notification # unit-level outbox/checker tests
```

- Integration tests use in-memory SQLite via `test_app()` / `test_app_with_pool()`
- `test_app_with_pool()` returns the pool for DB verification
- FK enforcement is ON in all test pools

## Coding Standards

- Error handling: use domain error types (MonitorError, IntegrationError), propagate with `?`
- No `.unwrap()` or `.expect()` in production code -- enforced by `clippy::unwrap_used`
- `.unwrap()` and `.expect()` allowed in test code only
- Services compute business values, repos only persist
- ScheduleType::next_occurrence_after() owns cron/interval computation

## Agent Contribution Policy

When an agent contributes code:

1. Disclose that the contribution is agent-assisted
2. List which files were consulted and what was changed
3. Run `just fmt && just lint && just test` before submitting
4. If uncertain whether a change is appropriate, stop and ask
