# Codebase Concerns

**Analysis Date:** 2026-04-01

## Tech Debt

**BigInt JSON Parsing Workaround:**
- Issue: Custom `parseJsonWithBigInt` function (with "TODO: Figure out a more permanent solution") is a temporary workaround for handling large integers in API responses
- Files: `ui/src/services/api.ts` (lines 27-53)
- Impact: Tight coupling to a regex-based approach that may not handle all edge cases; brittle if JSON structure changes; requires manual maintenance if precision requirements evolve
- Fix approach: Consider implementing a proper JSON reviver strategy or negotiating with backend API to return timestamps as strings, then parsing with a dedicated library like `big.js` or `decimal.js`

**Hardcoded API Base URL Path Resolution:**
- Issue: `~quent/types` alias points to local example simulator bindings instead of dynamic server-provided types
- Files: `ui/vite.config.ts` (lines 65-67)
- Impact: Type bindings are tightly coupled to example code; if backend API changes, types must be manually regenerated and committed; blocks runtime type flexibility
- Fix approach: Implement a type-generation step that runs against the actual backend API during build, or provide a type endpoint from the server

**Silently Failed Timeline Expansion:**
- Issue: Catch block in `useBulkTimelines` silently swallows errors without logging or fallback notification
- Files: `ui/src/hooks/useBulkTimelines.ts` (lines 174-176)
- Impact: Users won't know if timeline expansion failed; individual ResourceTimeline components will self-fetch but may not indicate state; difficult to debug in production
- Fix approach: Add proper error logging (non-blocking), optionally show a subtle UI indicator that data is stale/self-fetched

**Manual Chart Synchronization:**
- Issue: Axis pointer sync relies on DOM querying and manual crosshair event broadcasting to keep charts in sync
- Files: `ui/src/lib/timeline.utils.ts` (lines 374-429, 315-372)
- Impact: Fragile to DOM structure changes; relies on undocumented ECharts internals (`_echarts_instance_` attribute); coupling between charts makes refactoring risky
- Fix approach: Consider using ECharts' built-in group/connect API more exclusively, or implement a centralized state-based sync pattern

**Global Color Assignment State:**
- Issue: Color palette uses global mutable state (`colorAssignments`, `usedIndices`) with hash-based collision handling
- Files: `ui/src/services/colors.ts` (lines 76-100)
- Impact: Color assignments can change depending on order of operations; not deterministic across page reloads; palette exhaustion silently allows collisions; difficult to test in isolation
- Fix approach: Move to a deterministic, seeded hash or implement a proper collision-free assignment algorithm (e.g., graph coloring); store assignment state in React context or Jotai atom instead of module-level globals

## Performance Bottlenecks

**Large Component Files:**
- Files with potential complexity:
  - `ui/src/components/ui/tree-table.tsx` (812 lines) - Accordion tree with nested state management
  - `ui/src/lib/timeline.utils.ts` (647 lines) - Multiple concerns: binning, axis sync, color mapping
  - `ui/src/components/ui/tree-view.tsx` (524 lines) - Drag-and-drop + expansion state
  - `ui/src/components/timeline/TimelineController.tsx` (378 lines) - Chart setup + zoom control
  - `ui/src/components/timeline/Timeline.tsx` (376 lines) - Chart rendering with ECharts

- Impact: Long files are harder to test, understand, and maintain; refactoring is risky; multiple concerns make code reuse difficult
- Improvement path: Extract tree rendering logic into smaller composable utilities; move axis sync registration into a custom hook; separate chart configuration from rendering

**Expensive Tree Transformations:**
- Issue: `transformResourceTree`, `collectVisibleEntries`, and bulk parameter building happen on every parent re-render
- Files: `ui/src/components/QueryResourceTree.tsx` (lines 58-59), `ui/src/hooks/useBulkTimelines.ts` (lines 78-89)
- Impact: Large resource trees (hundreds of items) may cause re-render performance issues; no memoization of intermediate tree states
- Improvement path: Implement persistent data structures or virtualized lists for large trees; cache tree transformations at atom level

**Unmemoized Chart Re-renders:**
- Issue: `EChartsOption` is regenerated on every render even when data hasn't changed
- Files: `ui/src/components/timeline/Timeline.tsx` (lines 63-96), `ui/src/components/timeline/TimelineController.tsx` (lines 78-93)
- Impact: ECharts internally diffs options but large series data (thousands of points) will be re-serialized on each render
- Improvement path: Add shouldComponentUpdate or React.memo with custom comparison; memoize series data separately

## Fragile Areas

**Chart DOM Querying in Timeline Utils:**
- Files: `ui/src/lib/timeline.utils.ts` (lines 315-324)
- Why fragile: Direct DOM queries for ECharts instances rely on undocumented internal attributes; chart removal/recreation during development breaks sync
- Safe modification: Ensure chart instances are registered at creation time; don't rely on DOM queries in production code; consider adding error boundaries
- Test coverage: No tests for chart synchronization logic; manual testing only

**ECharts Configuration Differences:**
- Issue: Controller chart uses `xAxisIndex: 1` (value-based) while resource timelines use `xAxisIndex: 0` (time-based), requiring custom pixel conversion
- Files: `ui/src/lib/timeline.utils.ts` (lines 381-449), `ui/src/components/timeline/TimelineController.tsx` (lines 85)
- Why fragile: Asymmetric axis configuration means any change to one chart's xAxis setup breaks the other; conversion logic (`convertToPixel`, `convertFromPixel`) has implicit assumptions
- Safe modification: Document xAxis index contract clearly; consider standardizing to time-based axes for both; add assertion tests for axis consistency
- Test coverage: Axis sync has no unit tests; only manual integration testing

**Async Error Handling Gaps:**
- Issue: Multiple async operations lack proper error handling or user feedback
- Files:
  - `ui/src/hooks/useBulkTimelines.ts` (line 174) - silent catch
  - `ui/src/routes/profile.engine.$engineId.query.$queryId.index.tsx` (line 16) - route loader unhandled promise rejection
  - `ui/src/components/QueryResourceTree.tsx` (lines 91-119) - useQuery without error boundary
- Why fragile: Failed API calls may leave UI in partial/stale state; no error toast or fallback UI
- Safe modification: Wrap async operations with error boundaries; add error callbacks to useQuery; implement retry logic with exponential backoff
- Test coverage: No error scenario tests; only happy path tested

**Jotai Atom Lifecycle Management:**
- Issue: Multiple atoms managing overlapping state with debouncing and zoom sync
- Files: `ui/src/atoms/timeline.ts`, `ui/src/hooks/useBulkTimelines.ts`, `ui/src/components/QueryResourceTree.tsx`
- Why fragile: Complex atom dependencies (zoomRangeAtom → debouncedZoomRangeAtom → bulk fetch) mean changes in one place ripple through the system; no clear dependency visualization
- Safe modification: Document atom graph clearly; consider consolidating related atoms into a single family; use React Query as primary state source
- Test coverage: Atom behavior tested only indirectly through component tests

## Test Coverage Gaps

**Timeline Utilities Not Tested:**
- What's not tested:
  - `buildBinnedTimelineSeries` (647 line file, critical for data transformation)
  - `getAdaptiveNumBins`, `getLongEntitiesThreshold` (bin calculation logic)
  - `nanosToMs` (BigInt time conversion)
  - Axis sync registration/unregistration
  - Chart connection and group management
- Files: `ui/src/lib/timeline.utils.ts`
- Risk: Data transformation bugs won't surface until production; precision loss in nanosecond conversions could cause misalignment
- Priority: High - foundational utility code with no tests

**UI Component Integration Not Tested:**
- What's not tested:
  - TimelineController zoom/pan interaction (only 50 lines of manual test file)
  - Tree expansion/collapse with bulk fetch integration
  - DAGChart with ELK layout (234 lines, uses async layout calculation)
- Files: `ui/src/components/timeline/TimelineController.tsx`, `ui/src/components/dag/DAGChart.tsx`
- Risk: Interaction bugs, memory leaks from chart instances, stale closures in event handlers
- Priority: Medium-High - user-facing components

**Color Assignment Logic Not Tested:**
- What's not tested: Collision detection, palette saturation, hash determinism
- Files: `ui/src/services/colors.ts`
- Risk: Duplicate colors assigned under certain orderings; non-deterministic behavior across sessions
- Priority: Medium - affects visual output consistency

**Error Path Coverage:**
- What's not tested: API errors, network failures, malformed responses, timeouts
- Files: `ui/src/services/api.ts`, `ui/src/hooks/useBulkTimelines.ts`
- Risk: Unknown behavior on error; silent failures
- Priority: High - reliability critical

**Test File Count:** 4 test files for 67 source files (~6% coverage ratio)
- Tested modules: api service, example route, QueryResourceTree
- Untested modules: hooks (except indirectly), utils (except api parsing), all components except QueryResourceTree

## Security Considerations

**BigInt Regex Vulnerability:**
- Risk: Regex-based JSON preprocessing could be exploited with crafted payloads; no validation of generated JSON string before parsing
- Files: `ui/src/services/api.ts` (lines 35-45)
- Current mitigation: Regex is narrow in scope (16+ digit numbers); JSON.parse validates result
- Recommendations: Add input length limits; consider safer approaches like streaming JSON parser with BigInt support; add request/response size limits at proxy

**Dynamic Alias Resolution:**
- Risk: Type bindings pulled from local filesystem path instead of verified backend endpoint
- Files: `ui/vite.config.ts` (line 67)
- Current mitigation: Build-time binding (can't be manipulated at runtime); developer-controlled
- Recommendations: Add integrity check for bundled types; log type versions in build artifacts; implement fallback to server-provided types if available

**Silent Error Swallowing:**
- Risk: Errors in data fetching could mask data integrity issues; users may work with stale/incomplete data
- Files: `ui/src/hooks/useBulkTimelines.ts` (line 174)
- Current mitigation: Individual components self-fetch if bulk fails
- Recommendations: Log all errors; implement telemetry/monitoring for fetch failures; add user-facing warnings

## Scaling Limits

**DOM Chart Instances:**
- Current capacity: Dozens of simultaneous timeline charts before rendering degradation (ECharts can handle ~1000 series per chart but with noticeable lag at 500+)
- Limit: Opening 20+ timeline charts simultaneously will cause frame drops; chart synchronization becomes expensive
- Scaling path: Implement virtual scrolling for timeline list; lazy-load chart instances; consider canvas rendering or WebGL backend; implement requestIdleCallback for non-critical updates

**Bulk Timeline Request Size:**
- Current capacity: ~100 resources per bulk request (arbitrary limit based on API design, not tested)
- Limit: API may reject requests with too many entries; network timeout risk
- Scaling path: Implement request batching/pagination; add client-side request size validation; split bulk requests if exceeding threshold

**Color Palette Exhaustion:**
- Current capacity: 9 colors (echarts palette) before collisions occur
- Limit: >9 unique timeline series will reuse colors; user confusion with 20+ categories
- Scaling path: Implement dynamic palette generation; use pattern fill + color; implement color distribution algorithm

## Dependencies at Risk

**ECharts Version Lock:**
- Risk: `echarts@5.6.0` pinned; major updates could introduce breaking changes in chart sync behavior
- Impact: Manual `convertToPixel` calls may break with new ECharts versions; undocumented `_echarts_instance_` attribute may change
- Migration plan: Upgrade test coverage for chart sync before updating; consider using public ECharts API instead of internals; evaluate alternatives like Plotly.js or D3.js

**Radix UI Accordion Dependency:**
- Risk: Tree-table uses Radix accordion primitive; expansion state management coupled to Radix API
- Impact: Switching to custom accordion would require significant refactor of tree-table expansion logic
- Migration plan: Abstract accordion behind a custom hook interface; decouple tree state from Radix state management

**ELK.js for DAG Layout:**
- Risk: `elkjs@0.11.1` is WASM-based layout engine; loading and initialization could be slow on large graphs
- Impact: DAG rendering may stall on graphs >500 nodes
- Migration plan: Implement simpler layout algorithm; consider server-side layout; use Cytoscape.js instead

## Missing Critical Features

**No Offline Support:**
- Problem: Application is fully online-dependent; no caching beyond React Query's default 5-minute window
- Blocks: Users cannot view previously loaded queries offline; no service worker

**No Undo/Redo:**
- Problem: Tree expansion, zoom changes, and filter selections have no undo mechanism
- Blocks: Users must re-configure state manually after accidental changes

**No Export Functionality:**
- Problem: Timelines and DAGs cannot be exported as images, PDF, or CSV
- Blocks: Users cannot share visualizations or include in reports

**No Real-time Updates:**
- Problem: All data is static snapshot from load time; no WebSocket subscription for live query status
- Blocks: Cannot show in-progress query execution; users must manually refresh

**No Accessibility (a11y) Pass:**
- Problem: Timeline charts (ECharts) are not keyboard navigable; color-only distinctions violate WCAG
- Blocks: Screen reader users, keyboard-only users cannot use visualizations

## Missing Test Infrastructure

**No E2E Test Suite:**
- Currently: Only unit/integration tests with React Testing Library
- Needed: Cypress or Playwright tests for full user workflows
- Critical paths: Query selection → timeline load → zoom interaction → expand tree

**No Performance Benchmarks:**
- Currently: No performance tests or benchmarks
- Needed: Baseline metrics for large timeline rendering, tree transformation speed, API response size
- Critical metrics: Time-to-interactive, first paint, memory footprint with 1000+ series points

**No Visual Regression Tests:**
- Currently: No visual snapshots or screenshot comparison tests
- Needed: Catch unintended styling changes in Timeline, DAGChart, and other visual components

---

*Concerns audit: 2026-04-01*
