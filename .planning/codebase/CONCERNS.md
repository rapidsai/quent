# Codebase Concerns

**Analysis Date:** 2026-03-25

## Critical Issues Requiring Attention

### Unwrap Race Condition in Collector Service

**Issue:** Unsafe unwrap in concurrent collector service
**Files:** `crates/collector/server/src/server.rs:81`
**Impact:** Potential panic in production when handling gRPC stream events
**Problem:**
```rust
let exporter = if exporters.contains_key(&application_id) {
    Arc::clone(&exporters.get(&application_id).unwrap())  // Line 81 - RACE CONDITION
```
The code checks `contains_key()` then calls `get()` with `unwrap()`. Between these operations, another thread could remove the key from the DashMap, causing a panic. This is a TOCTOU (Time-of-Check-Time-of-Use) vulnerability in concurrent code.

**Fix Approach:** Replace with DashMap's `entry()` API or use `or_insert_with()` to atomically create/retrieve the exporter in a single operation.

---

## Tech Debt

### Resource Cleanup Not Implemented

**Area:** Collector Service Exporter Lifecycle
**Files:** `crates/collector/server/src/server.rs:26`
**Issue:** TODO marker indicates unimplemented cleanup
```rust
// TODO(johanpel): clean up exporter after timeout or application end.
```
**Impact:** Long-running servers accumulate exporters for inactive applications, causing memory growth
**Fix Approach:** Implement a timeout-based cleanup mechanism (e.g., remove exporters inactive for 30 minutes) or track application lifecycle explicitly

### Client Disconnection Handling Incomplete

**Area:** gRPC Stream Error Handling
**Files:** `crates/collector/server/src/server.rs:125`
**Issue:** Incomplete error handling when clients disconnect
```rust
// TODO(johanpel): a client disconnecting (abruptly?) may result in entering this branch.
// We should clean up here, but the todo is to figure out what else can go wrong.
```
**Impact:** Unclear error semantics during disconnection; potential resource leaks if cleanup path misses edge cases
**Priority:** Medium - code currently handles the case but needs clarification for maintenance

### Unbounded Channel Buffer

**Area:** Collector Client
**Files:** `crates/collector/client/src/lib.rs:78`
**Issue:** TODO indicates potential unbounded queue
```rust
// TODO(johanpel): consider unbounded
```
**Impact:** Could cause memory exhaustion if event production exceeds export throughput
**Fix Approach:** Evaluate and set appropriate channel bounds; implement backpressure handling

---

## Type Safety Issues

### Excessive Type Casting in UI

**Area:** TypeScript Type System
**Files:** `ui/src/lib/queryBundle.utils.ts`, `ui/src/lib/timeline.utils.ts`
**Issue:** 7 instances of `as unknown` type assertions circumvent TypeScript safety
**Examples:**
- `ui/src/lib/queryBundle.utils.ts:69`: `(val as unknown as Record<string, unknown>)[0]`
- `ui/src/lib/timeline.utils.ts:303`: `instance as unknown as EChartsInstance`

**Impact:** Masks type errors; could lead to runtime errors if assumptions about structure are wrong
**Fix Approach:** Replace assertions with proper type guards or discriminated unions. Example:
```typescript
// Instead of: (val as unknown as Record<string, unknown>)
// Use proper narrowing:
function unwrapTaggedValue(val: unknown): StatValue {
  if (typeof val === 'object' && val !== null && !Array.isArray(val)) {
    return /* safe access */
  }
}
```

### API BigInt Parsing Workaround

**Area:** JSON Serialization
**Files:** `ui/src/services/api.ts:27-53`
**Issue:** Custom BigInt parsing regex as temporary solution
**Impact:** Maintenance burden; regex-based parsing is fragile; large integers in unexpected positions could be missed
**Fix Approach:** Use a proper JSON reviver or switch to a serialization format with native BigInt support (e.g., MessagePack)

---

## Performance Concerns

### Complex Component Implementation

**Area:** Timeline Rendering
**Files:** `ui/src/components/timeline/Timeline.tsx` (376 lines), `ui/src/lib/timeline.utils.ts` (630 lines)
**Issue:** Large monolithic component with intricate state management for ECharts synchronization
**Impact:** Difficult to test; risky refactoring; potential perf regressions from changes
**Note:** The `CHART_GROUP` synchronization pattern across 3+ timeline charts is tightly coupled

### Large Tree Component

**Area:** Resource Tree UI
**Files:** `ui/src/components/ui/tree-table.tsx` (812 lines)
**Issue:** Extensive primitive component with complex nested rendering logic
**Impact:** Testing gaps; manual expansion/collapse state management; accessibility concerns
**Mitigation:** Consider breaking into smaller, composable sub-components for row rendering

### Resource Timeline Binned Calculation

**Area:** Timeline Data Processing
**Files:** `crates/analyzer/src/timeline/binned/resource.rs` (865 lines)
**Issue:** Very large analyzer module handling complex FSM state transitions and capacity calculations
**Impact:** High cyclomatic complexity; error handling could miss edge cases
**Note:** Contains 2 instances of `.ok()` that silently drop errors

---

## Test Coverage Gaps

### Minimal UI Test Coverage

**Files:** Only 2 test files for entire UI:
- `ui/src/test/example.test.tsx` - Generic example
- `ui/src/routes/profile.index.test.tsx` - Route-level only
- `ui/src/services/api.test.ts` - Focused on BigInt parsing only

**Untested Components:**
- `TimelineController.tsx` (378 lines) - No tests
- `Timeline.tsx` (376 lines) - No tests
- `TimelineTooltip.tsx` (329 lines) - No tests
- `ResourceTimeline.tsx` (244 lines) - No tests
- `QueryResourceTree.tsx` (244 lines) - No tests
- DAG chart components - No tests
- Hook implementations (`useBulkTimelines`, `useQueryBundle`, etc.) - No tests

**Impact:** Cannot safely refactor complex timeline logic; visual regressions undetectable
**Priority:** High - Timeline components are critical and under active development

### Rust Test Coverage

**Status:** 7 test modules across crates
**Concern:** Analyzer modules with complex FSM/resource state logic have limited edge case coverage

---

## Known Design Limitations

### Instance Name Not Captured in Timeline Requests

**Area:** Timeline Data Collection
**Files:** `crates/ui/src/timeline/request.rs:48`
**Issue:** TODO indicates missing context
```rust
// TODO(johanpel): instance name
```
**Impact:** Limited ability to correlate timeline data with specific resource instances
**Severity:** Medium - affects data enrichment but not correctness

### Capacity Type Rate Not Fully Modeled

**Area:** Resource Capacity System
**Files:** `crates/analyzer/src/resource/mod.rs:83`
**Issue:** Rate capacity variant may need additional design
```rust
// TODO(johanpel): the rate capacity type may need an additional variant
```
**Impact:** Could affect correctness of rate-based metrics if new variant patterns emerge
**Priority:** Medium - depends on future feature requirements

---

## Fragile Areas Requiring Careful Changes

### FSM Transition Validation

**Files:** `crates/analyzer/src/fsm/runtime.rs` (411 lines)
**Concern:** Complex state machine transition logic with multiple error conditions
**Errors Defined:**
- `IncompleteEntity` - Missing FSM type/instance names (lines 134-137)
- `IncompleteEntity` - No type name (line 99)
**Safe Modifications:**
- Preserve all error conditions during refactoring
- Add test cases for each error path
- Ensure state transitions remain atomic

### Resource Collection Constraint Handling

**Files:** `crates/analyzer/src/resource/collection.rs` (360 lines)
**Concern:** Blocking/non-blocking channel semantics noted as incomplete
```rust
// TODO(johanpel): see CapacityType and consider blocking/non-blocking channels
```
**Risk:** Design decisions about concurrency model not finalized
**Safe Approach:** Avoid finalizing API until requirements are clear

### Timeline Bin Performance Optimization

**Files:** `crates/analyzer/src/timeline/binned/resource.rs:66`
**Concern:** Performance acknowledged as acceptable but suboptimal
```rust
// TODO(johanpel): perf is fine for now but at some point we want to consider preventing all the hashmaps.
```
**Current State:** Multiple HashMap allocations per timeline calculation
**Future Path:** Profile before optimizing; changes could affect accuracy

---

## Incomplete Feature Implementation

### Query Plan Fetching Logic Uncertain

**Area:** UI Query Initialization
**Files:** `ui/src/components/QueryPlan.tsx:37`
**Issue:** Design decision marked as unresolved
```typescript
// TODO: Currently fetching root plan when bundle loads - is this correct?
```
**Impact:** Query plan data fetch strategy may change; dependent features could be affected
**Resolution Needed:** Clarify whether root plan should be fetched eagerly or lazily

### DAG Node Selection Routing Unclear

**Area:** Query Visualization Navigation
**Files:** `ui/src/routes/profile.engine.$engineId.query.$queryId.node.$nodeId.tsx:11`
**Issue:** Route behavior identical to parent route
```typescript
// TODO: This does the same thing as the /query/$queryId route, figure out what happens when selecting nodes in the DAG
```
**Impact:** Inconsistent navigation UX; unclear component purpose
**Fix Approach:** Either merge routes or implement distinct node-specific behavior

---

## Dependencies at Risk

### No Active Unsafe Code

**Status:** Codebase free of explicit `unsafe {}` blocks
**Assessment:** Good security posture

### Exporter Lifecycle Management

**Risk:** No automatic cleanup of idle exporters
**Current Mitigation:** Manual cleanup on stream error
**Scaling Risk:** High-throughput system with many short-lived applications could accumulate exporters

---

## Timestamp Edge Case Known

**Files:** `crates/time/src/lib.rs:16`
**Issue:** u64::MAX reserved but not enforced
```rust
// TODO(johanpel): u64::MAX should be excluded as a valid timestamp because it
```
**Impact:** Edge case in binning calculations revealed during testing
**Files Affected:** `crates/time/src/bin.rs` - test cases explore u64::MAX range
**Mitigation:** Add explicit validation in timestamp creation

---

*Concerns audit: 2026-03-25*
