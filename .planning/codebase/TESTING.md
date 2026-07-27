# Testing Patterns

**Analysis Date:** 2026-07-08

## Test Frameworks

**Rust:**
- Built-in `cargo test` harness; `#[tokio::test]` for async tests
- `trybuild = "1"` for macro compile-fail tests (`crates/model/Cargo.toml`)
- `tempfile = "3"` for filesystem fixtures (dev-dep in `crates/exporter/ndjson`, `crates/instrumentation`, `crates/open`)
- `criterion` 0.5 + `pprof` (flamegraphs) for benchmarks (`crates/instrumentation/benches/event_emit.rs`)

**UI (TypeScript):**
- Vitest 4 (`ui/vitest.config.ts`), jsdom environment, `globals: true`
- `@testing-library/react` + `@testing-library/user-event` + `@testing-library/jest-dom/vitest`
- MSW 2 for API mocking (`ui/src/test/mocks/server.ts`, `ui/src/test/mocks/handlers.ts`)
- Playwright for E2E (`ui/playwright.config.ts`, tests in `ui/e2e/`)

**Python:**
- pytest (>=8, via `pixi.toml`); typed tests checked with `ty` (e.g. `# ty:ignore[unsupported-operator]` in `domains/query_engine/tests/python/test_query_engine.py`)

## Run Commands

```bash
# Rust — all tests (what CI runs, .github/workflows/rust.yml)
pixi run cargo test --workspace --all-features --locked --all-targets

# Rust — opt-in crates excluded from default-members (test-clock override)
cargo test -p <crate>          # e.g. domains/query_engine/tests/fixed

# UI (from ui/)
pnpm test                      # vitest watch mode
pnpm test:run                  # single run
pnpm test:coverage             # with v8 coverage
pnpm test:e2e                  # playwright
pnpm ci:check                  # full CI pipeline: format, lint, typecheck, coverage, audit, e2e, build
```

## Test File Organization

**Rust:**
- Unit tests: inline `#[cfg(test)] mod tests { use super::*; ... }` at the bottom of source files (~40 files, e.g. `crates/exporter/ndjson/src/lib.rs`, `crates/instrumentation/src/lib.rs`, `crates/analyzer/src/resource/tree.rs`)
- Integration tests: `tests/` directory per crate with descriptive snake_case names (`crates/model/tests/entity_and_events.rs`, `crates/instrumentation/tests/collector_roundtrip.rs`)
- Shared integration fixtures: `tests/common/mod.rs` with `#![allow(dead_code)]` (`crates/instrumentation/tests/common/mod.rs`)
- Compile-fail cases: `crates/model/tests/compile_fail/*.rs` with paired `.stderr` expectation files, driven by `crates/model/tests/macro_corner_cases.rs`
- Cross-language bridge tests: `domains/query_engine/tests/cpp/` and `domains/query_engine/tests/python/` (workspace member crates)

**UI:**
- Co-located `*.test.ts(x)` next to sources (e.g. `ui/src/components/QueryResourceTree.test.tsx`, `ui/packages/@quent/utils/src/formatters.test.ts`)
- Vitest include: `src/**/*.{test,spec}.{ts,tsx}` and `packages/@quent/*/src/**/*.{test,spec}.{ts,tsx}`
- Test infrastructure: `ui/src/test/` (`setup.ts`, `test-utils.tsx`, `mocks/`)
- E2E specs: `ui/e2e/*.spec.ts` with `ui/e2e/global-setup.ts`

## Test Structure

**Rust unit test pattern:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestEvent;
    impl EntityEvent for TestEvent {
        const NAME: &'static str = "TestEvent";
    }

    #[tokio::test]
    async fn push_after_shutdown_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mut exporter = NdjsonExporter::try_new::<TestEvent>(...).await.unwrap();
        ...
        assert!(matches!(exporter.push(...).await, Err(ExporterError::Shutdown)));
    }
}
```
(from `crates/exporter/ndjson/src/lib.rs`)

**Patterns:**
- Test names are behavior sentences in snake_case: `push_after_shutdown_errors`, `entity_event_attributes_populated`, `resolve_import_path_handles_dir_and_file`
- `.unwrap()` freely in tests; `assert!(matches!(...))` for error-variant checks; assertion messages explain the invariant
- Minimal local fixture types (`TestModel`, `TestEvent`) defined per test module implementing the needed traits

**UI test pattern:**
```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/mocks/server';
import { screen, renderWithRouter, waitFor } from '@/test/test-utils';

describe('EngineSelectionPage', () => {
  beforeEach(() => {
    server.use(http.get(`${API_BASE}/engines`, () => HttpResponse.json([...])));
  });

  it('renders the page title and description', async () => {
    renderWithRouter({ initialPath: '/profile' });
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
    });
  });
});
```
(from `ui/src/routes/profile.index.test.tsx`)

- Nested `describe` blocks group scenarios; `it` names are behavior sentences
- Pure-function tests use plain `describe`/`it`/`expect` with section-divider comments (`ui/packages/@quent/utils/src/formatters.test.ts`)

## Mocking

**Rust:** No mocking framework. Use real implementations with `tempfile::tempdir()` for filesystem, local trait-impl fixture structs for models/events. Deterministic time via the `quent-time/__test-clock-override` feature (opt-in crates only, see root `Cargo.toml` comments).

**UI — MSW (network layer):**
- Server: `ui/src/test/mocks/server.ts`; default handlers: `ui/src/test/mocks/handlers.ts`
- Lifecycle in `ui/src/test/setup.ts`: `server.listen({ onUnhandledRequest: 'warn' })` in `beforeAll`, `server.resetHandlers()` + `cleanup()` in `afterEach`, `server.close()` in `afterAll`
- Per-test overrides via `server.use(http.get(...))` in `beforeEach`

**UI — browser API stubs (`ui/src/test/setup.ts`):**
- `window.matchMedia` mocked with `vi.fn()` (for ThemeToggle)
- `ResizeObserver` class mock and `Element.prototype.scrollIntoView = vi.fn()` (for Radix UI)

**What to Mock:** network requests (MSW), browser APIs missing from jsdom
**What NOT to Mock:** React Query/Router — use real providers via `ui/src/test/test-utils.tsx` helpers

## Fixtures and Test Utilities

**UI render helpers (`ui/src/test/test-utils.tsx`):**
- `renderWithQuery(ui)` — wraps in a fresh `QueryClient` (retry off, `gcTime: 0`) to prevent state leakage
- `renderHookWithQuery(hook)` — same for hooks
- `renderWithRouter({ initialPath })` — full app render with memory history against the real `routeTree.gen.ts`
- Re-exports all of `@testing-library/react` plus `userEvent`

**Rust fixtures:** `tests/common/mod.rs` modules (e.g. `crates/instrumentation/tests/common/mod.rs` provides `TestModel`/`TestEvent`)

## Coverage

**Requirements:** No numeric threshold enforced; UI CI runs `pnpm test:coverage` and uploads `ui/coverage/` as an artifact (`.github/workflows/ui.yml`). No Rust coverage tooling configured.

**UI coverage config (`ui/vitest.config.ts`):** provider `v8`; reporters `text`, `json`, `html`, `cobertura`; excludes `src/test/`, `src/routeTree.gen.ts`, `**/*.d.ts`, `**/*.config.*`. JUnit XML emitted to `ui/junit.xml`.

## Test Types

**Unit tests:** Rust inline `#[cfg(test)]` modules; UI pure-function tests in `ui/packages/@quent/*/src/**`
**Integration tests:** Rust `tests/` dirs (macro expansion in `crates/model/tests/`, exporter roundtrips in `crates/instrumentation/tests/collector_roundtrip.rs`); UI route-level tests with MSW + real router
**Compile-fail tests:** trybuild in `crates/model/tests/macro_corner_cases.rs` against `tests/compile_fail/*.rs` + `.stderr`
**E2E tests:** Playwright, chromium only, `ui/e2e/smoke.spec.ts`; builds and serves the app via `webServer` config unless `PLAYWRIGHT_BASE_URL` set; CI retries 2, single worker
**Cross-language tests:** C++/Python bridge crates under `domains/query_engine/tests/{cpp,python}/`; Python pytest in `domains/query_engine/tests/python/test_query_engine.py`
**Benchmarks:** criterion in `crates/instrumentation/benches/event_emit.rs` (see its README.md)

## Common Patterns

**Rust async testing:**
```rust
#[tokio::test]
async fn push_after_shutdown_errors() { ... }
```

**Rust error testing:**
```rust
assert!(matches!(result, Err(ExporterError::Shutdown)));
```

**UI async testing:**
```typescript
await waitFor(() => {
  expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
});
```

**Python error testing:**
```python
with pytest.raises(TypeError):
    uuid_a < uuid_b  # ty:ignore[unsupported-operator]
```

## CI Requirements

- Rust: fmt check, clippy `-D warnings`, `cargo test --workspace --all-features --locked --all-targets`, release build, `cargo deny check` (`.github/workflows/rust.yml`)
- UI: format check, ESLint, typecheck, tests with coverage, `pnpm audit --audit-level=high`, Playwright e2e, production build (`.github/workflows/ui.yml`, or `pnpm ci:check` locally)
- New components should include accompanying tests (`CONTRIBUTING.md`)

---

*Testing analysis: 2026-07-08*
