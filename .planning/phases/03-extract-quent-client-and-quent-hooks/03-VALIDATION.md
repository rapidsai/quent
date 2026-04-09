---
phase: 03
slug: extract-quent-client-and-quent-hooks
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-09
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | vitest |
| **Config file** | `ui/vitest.config.ts` + `ui/vitest.workspace.ts` |
| **Quick run command** | `cd ui && pnpm --filter @quent/client exec tsc --noEmit && pnpm --filter @quent/hooks exec tsc --noEmit` |
| **Full suite command** | `cd ui && pnpm test:run` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `pnpm --filter @quent/client exec tsc --noEmit && pnpm --filter @quent/hooks exec tsc --noEmit`
- **After every plan wave:** Run `cd ui && pnpm test:run`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | CLIENT-01 | typecheck | `pnpm --filter @quent/client exec tsc --noEmit` | ❌ W0 | ⬜ pending |
| 03-01-02 | 01 | 1 | CLIENT-02 | typecheck | `pnpm --filter @quent/client exec tsc --noEmit` | ❌ W0 | ⬜ pending |
| 03-01-03 | 01 | 1 | CLIENT-03, CLIENT-04, CLIENT-05 | typecheck | `pnpm --filter @quent/client exec tsc --noEmit` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 1 | HOOKS-01, HOOKS-04 | typecheck | `pnpm --filter @quent/hooks exec tsc --noEmit` | ❌ W0 | ⬜ pending |
| 03-02-02 | 02 | 1 | HOOKS-02, HOOKS-03 | typecheck | `pnpm --filter @quent/hooks exec tsc --noEmit` | ❌ W0 | ⬜ pending |
| 03-03-01 | 03 | 2 | CLIENT-01..05, HOOKS-01..04 | unit+e2e | `cd ui && pnpm test:run` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `ui/packages/@quent/client/src/index.ts` — populated barrel (replaces empty scaffold)
- [ ] `ui/packages/@quent/hooks/src/index.ts` — populated barrel (replaces empty scaffold)
- [ ] `ZoomRange` type extracted to `@quent/utils` before any hook moves (pre-condition per RESEARCH.md)

*Wave 0 = pre-conditions that must exist before Wave 1 tasks can typecheck.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Jotai Provider per-query scoping resets state when switching queries | HOOKS-04 | Requires running app + navigating between query pages | Open app, load query A (DAG node selected), navigate to query B — confirm selected node is reset |
| DAG node selection updates both the DAG highlight AND timeline filter | HOOKS-03 | Cross-component state interaction | Select a node in the DAG; confirm timeline re-fetches with operator filter |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
