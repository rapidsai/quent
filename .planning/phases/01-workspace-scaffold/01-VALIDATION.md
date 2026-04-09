---
phase: 1
slug: workspace-scaffold
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-01
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Vitest 4.0.18 |
| **Config file** | `ui/vitest.config.ts` (existing); `ui/vitest.workspace.ts` (created in INFRA-05) |
| **Quick run command** | `pnpm typecheck` (from `ui/`) |
| **Full suite command** | `pnpm test:run` (from `ui/`) |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `pnpm typecheck`
- **After every plan wave:** Run `pnpm test:run`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 1-xx-01 | TBD | 1 | INFRA-01 | smoke | `pnpm install && pnpm why @quent/utils` | ❌ W0 | ⬜ pending |
| 1-xx-02 | TBD | 1 | INFRA-02 | type-check | `pnpm typecheck` | ✅ | ⬜ pending |
| 1-xx-03 | TBD | 1 | INFRA-03 | smoke | `pnpm --filter @quent/utils build` | ❌ W0 | ⬜ pending |
| 1-xx-04 | TBD | 1 | INFRA-04 | smoke | `pnpm why react` | ✅ | ⬜ pending |
| 1-xx-05 | TBD | 2 | INFRA-05 | automated | `pnpm test:run` | ❌ W0 | ⬜ pending |
| 1-xx-06 | TBD | 1 | INFRA-06 | automated | `grep "@source" ui/src/index.css` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `ui/vitest.workspace.ts` — to be created as part of INFRA-05; verify by running `pnpm test:run` after creation
- [ ] Per-package `vitest.config.ts` stubs — may be needed if Vitest workspace glob requires at least one match per package

*Wave 0 artifacts are structural scaffolding — no test stubs needed beyond Vitest workspace config.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `pnpm dev` starts, app renders normally | INFRA-04 | Requires browser check | Run `pnpm dev` from `ui/`; verify app loads in browser without console errors |
| Single hoisted React instance | INFRA-04 | Output reading required | Run `pnpm why react` from `ui/`; verify only one version listed |
| `pnpm install` resolves workspace packages | INFRA-01 | Requires lockfile state | Run `pnpm install` from `ui/`; verify exits 0 and `node_modules/@quent/*` symlinks exist |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
