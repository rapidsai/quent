---
phase: 4
slug: extract-quent-components-and-migrate-app-shell
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-13
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | vitest |
| **Config file** | `ui/vitest.config.ts` |
| **Quick run command** | `pnpm --filter ui test --run` |
| **Full suite command** | `pnpm --filter ui test --run && pnpm build` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `pnpm --filter ui test --run`
- **After every plan wave:** Run `pnpm --filter ui test --run && pnpm build`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 4-01-01 | 01 | 1 | COMP-01 | build | `pnpm --filter @quent/components build` | ✅ | ⬜ pending |
| 4-01-02 | 01 | 1 | COMP-02 | unit | `pnpm --filter ui test --run` | ✅ | ⬜ pending |
| 4-02-01 | 02 | 2 | MIG-01 | grep | `grep -r "@/components/" ui/src/ \| wc -l` | ✅ | ⬜ pending |
| 4-02-02 | 02 | 2 | MIG-02 | build | `pnpm build` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| DAG and timeline render correctly in running app | COMP-07 | Visual rendering requires browser | Start dev server, navigate to DAG view, verify chart renders with correct colors |
| Tailwind styles correct in production mode | MIG-03 | Requires `vite preview` visual check | Run `pnpm build && pnpm preview`, verify no purged classes |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
