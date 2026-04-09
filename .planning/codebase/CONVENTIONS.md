# Coding Conventions

**Analysis Date:** 2026-04-01

## Naming Patterns

**Files:**
- React components: PascalCase (e.g., `QueryResourceTree.tsx`, `ThemeToggle.tsx`)
- Hooks: camelCase prefixed with `use` (e.g., `useBulkTimelineFetch.ts`, `useExpandedIds.ts`)
- Services/utilities: camelCase (e.g., `api.ts`, `formatters.ts`)
- Test files: Same as source with `.test.ts` or `.test.tsx` suffix (e.g., `api.test.ts`, `QueryResourceTree.test.tsx`)
- Atoms/state: camelCase (e.g., `timeline.ts`)
- Types: camelCase files with types exported as PascalCase (e.g., `types.ts`)

**Functions:**
- Regular functions: camelCase (e.g., `getRootResourceGroupId`, `parseJsonWithBigInt`)
- React components: PascalCase (e.g., `TimelineController`, `QueryResourceTree`)
- Hooks: camelCase with `use` prefix (e.g., `useBulkTimelines`, `useHighlightedItemIds`)
- Event handlers: camelCase with `on` or `handle` prefix (e.g., `onZoomChange`, `handleExpandChange`)

**Variables:**
- Constants: SCREAMING_SNAKE_CASE for module-level constants (e.g., `CONTROLLER_HEIGHT`, `API_BASE_URL`)
- Local variables: camelCase (e.g., `selectedTypes`, `rootResourceGroupId`)
- State values: camelCase (e.g., `durationSeconds`, `expandedIds`)
- Maps/collections: camelCase plural (e.g., `selectedTypes`, `handlers`)

**Types:**
- Exported types: PascalCase (e.g., `ZoomRange`, `QueryResourceTreeProps`)
- Type files: Keep types with relevant code where possible; top-level types in `src/types.ts`
- Interfaces: PascalCase (e.g., `TimelineCacheParams`, `BulkTimelineIdMeta`)
- Discriminated unions: PascalCase variants (e.g., `ResourceGroup`, `Resource`)

## Code Style

**Formatting:**
- Tool: Prettier
- Print width: 100 characters
- Tab width: 2 spaces
- No tabs, use spaces
- Trailing commas: ES5 (trailing commas where valid)
- Single quotes: Enabled
- Semi-colons: Required
- Arrow function parentheses: Avoid (parentheses omitted for single parameters)
- Line endings: LF

**Linting:**
- Tool: ESLint with TypeScript support
- Config: `eslint.config.js` uses flat config format
- Key rules enforced:
  - `no-console`: ERROR with allow list for `console.warn` and `console.error` only
  - `@typescript-eslint/no-unused-vars`: ERROR with pattern ignoring arguments/vars starting with `_`
  - `react-refresh/only-export-components`: WARN for Vite React refresh
  - React Hooks rules enforced (dependency arrays, hook ordering)
- Auto-generated files excluded: `dist`, `src/routeTree.gen.ts`

## Import Organization

**Order:**
1. External dependencies (React, @tanstack, jotai, etc.)
2. Absolute imports using `@/` path alias
3. Type imports using `type` keyword
4. Relative imports (rare, use `@/` instead)

**Path Aliases:**
- `@/` maps to `src/` directory
- Used in all imports except node_modules dependencies

**Example:**
```typescript
import { useState, useMemo } from 'react';
import { useQuery, keepPreviousData } from '@tanstack/react-query';
import { useAtomValue } from 'jotai';
import { useHighlightedItemIds } from '@/hooks/useHighlightedItemIds';
import { transformResourceTree } from '@/lib/timeline.utils';
import type { QueryBundle } from '~quent/types/QueryBundle';
```

## Error Handling

**Patterns:**
- Logging: `console.error` and `console.warn` only (console.log is forbidden)
- API errors: Handled via React Query error states with fallback UI
- Component errors: React error boundaries for fatal errors
- Async operations: Use `.catch()` or try-catch with async/await

**Examples from codebase:**
- `src/hooks/useQueryPlanVisualization.ts`: `console.error('Error generating tree data:', error)`
- API responses: Missing data renders gracefully (e.g., empty lists, null checks)

## Logging

**Framework:** console (console.warn and console.error only)

**Patterns:**
- Error messages: Use `console.error` for recoverable errors with context
- Warnings: Use `console.warn` for potentially problematic conditions
- Debug logging: Not used in production code
- All console.log calls are forbidden by ESLint

**Example:**
```typescript
if (error) {
  console.error('Error generating tree data:', error);
}
```

## Comments

**When to Comment:**
- File headers: SPDX license headers required at top of every file
  ```typescript
  // SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
  // SPDX-License-Identifier: Apache-2.0
  ```
- Complex logic: Comments explain "why" not "what"
- TODO/FIXME: Used to mark technical debt (e.g., `TODO: Figure out a more permanent solution`)
- Mock explanations: Comment explaining why mocks are used
- Non-obvious calculations: Add inline comments explaining calculation logic

**JSDoc/TSDoc:**
- Used for exported functions and interfaces
- Document parameters, return types, and special behavior
- Used extensively in test utilities and service functions

**Examples:**
```typescript
/**
 * Distributes a bulk timeline response into per-item Jotai atoms.
 * Skips entries whose status is not 'ok' or whose id has no meta mapping.
 */
export function applyBulkTimelineResponse(
  response: BulkTimelinesResponse,
  idToMeta: Map<string, BulkTimelineIdMeta>,
  store: ReturnType<typeof import('jotai').useStore>
): void { ... }
```

## Function Design

**Size:**
- Keep functions focused and small (typically under 50 lines)
- Extract complex logic into helper functions

**Parameters:**
- Use object destructuring for multiple parameters
- Avoid boolean parameters (use semantic prop names instead)
- Type all parameters explicitly

**Return Values:**
- Explicit return types on all functions
- Use discriminated unions for complex returns
- Null/undefined only when representing absence of data

**Example:**
```typescript
export function TimelineController({
  startTime,
  durationSeconds,
  height = CONTROLLER_HEIGHT,
  timelineData,
  onZoomChange,
}: TimelineControllerProps) { ... }
```

## Module Design

**Exports:**
- Named exports preferred (e.g., `export function X() {}`)
- Default exports only for React components
- Type exports use `type` keyword: `export type X = ...`

**Barrel Files:**
- Not commonly used; prefer direct imports to `src/` locations
- Test utilities exported from `@/test/test-utils`

**File Organization:**
- Keep related types with code (e.g., interfaces near usage)
- Top-level shared types in `src/types.ts`
- Service layer in `src/services/` (api.ts, formatters.ts, colors.ts)
- Utilities in `src/lib/` (timeline.utils.ts, resource.utils.ts)
- Hooks in `src/hooks/`
- Components in `src/components/`
- State atoms in `src/atoms/`

---

*Convention analysis: 2026-04-01*
