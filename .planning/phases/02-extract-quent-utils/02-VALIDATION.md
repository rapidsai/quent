---
phase: 2
slug: extract-quent-utils
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-01
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | vitest ^4.0.18 |
| **Config file** | `ui/vitest.config.ts` (app); `ui/vitest.workspace.ts` (workspace) |
| **Quick run command** | `cd ui && pnpm --filter @quent/utils exec tsc --noEmit` |
| **Full suite command** | `cd ui && pnpm test:run` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd ui && pnpm --filter @quent/utils exec tsc --noEmit`
- **After every plan wave:** Run `cd ui && pnpm test:run`
- **Before `/gsd:verify-work`:** Full suite must be green + `cd ui && pnpm build` succeeds
- **Max feedback latency:** ~10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | UTILS-01 | smoke | `cd ui && pnpm --filter @quent/utils exec tsc --noEmit` | ❌ Wave 0 | ⬜ pending |
| 02-01-02 | 01 | 1 | UTILS-02 | smoke | `cd ui && pnpm --filter @quent/utils exec tsc --noEmit` | ❌ Wave 0 | ⬜ pending |
| 02-01-03 | 01 | 1 | UTILS-03 | unit | `cd ui && pnpm test:run -- src/services/api.test.ts` | ✅ exists (import path update needed) | ⬜ pending |
| 02-01-04 | 01 | 1 | UTILS-04 | smoke | `cd ui && pnpm --filter @quent/utils exec tsc --noEmit` | ❌ Wave 0 | ⬜ pending |
| 02-01-05 | 01 | 1 | UTILS-05 | smoke | `cd ui && pnpm --filter @quent/utils exec tsc --noEmit` | ❌ Wave 0 | ⬜ pending |
| 02-02-01 | 02 | 2 | All | integration | `cd ui && pnpm build` | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `ui/src/services/api.test.ts` — update import of `parseJsonWithBigInt` from `'./api'` to `'@quent/utils'` (or verify it still resolves via api.ts re-export)
- [ ] No new test files needed — all requirements verified by typecheck (type-level API surface) and the existing unit test

*Existing vitest infrastructure covers all phase requirements. No new config files needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| App renders correctly in browser after migration | All (success criterion 4) | Requires visual browser check | Run `cd ui && pnpm dev`; open localhost; verify DAG chart, timeline, and resource tree render without console errors |
| JSDoc hover visible in editor | UTILS-03, UTILS-04, UTILS-05 | Requires IDE interaction | Open `ui/src/` file, hover over imported `cn`, `formatDuration`, `getColorForKey` — verify JSDoc tooltip appears |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
