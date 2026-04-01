# Coding Conventions

**Analysis Date:** 2026-03-25

## Naming Patterns

**Files:**
- TypeScript/React files: camelCase (e.g., `useQueryBundle.ts`, `api.ts`, `DAGChart.tsx`)
- Component files: PascalCase (e.g., `QueryPlan.tsx`, `ResourceColumn.tsx`)
- Utility files: camelCase (e.g., `timeline.utils.ts`, `resource.utils.ts`)
- Rust modules: snake_case with files matching module names (e.g., `fsm/mod.rs`, `tree.rs`)

**Functions:**
- camelCase consistently used in TypeScript (e.g., `fetchQueryBundle`, `parseJsonWithBigInt`, `renderWithQuery`)
- Async functions clearly named with action verbs (e.g., `fetchListEngines`, `calculateLayout`)
- React hooks prefixed with `use` (e.g., `useQueryBundle`, `useQueryPlanVisualization`)
- Query option factories suffixed with `QueryOptions` (e.g., `queryBundleQueryOptions`)
- Utility functions prefixed with action/purpose (e.g., `collectResourceTypesFromTree`, `getIconForType`)

**Variables:**
- camelCase for local and module-level variables
- UPPERCASE_SNAKE_CASE for constants (e.g., `DEFAULT_STALE_TIME`, `API_BASE_URL`)
- Prefixes for clarity: `set` for setters, `is`/`has` for boolean checks
- Atom state variables: camelCase ending in `Atom` (e.g., `selectedPlanIdAtom`, `hoveredWorkerIdAtom`)

**Types:**
- PascalCase for all type definitions (e.g., `QueryBundle`, `EntityRef`, `QueryPlanNodeData`)
- Interfaces and type aliases follow same naming convention
- Generic type parameters: single uppercase letters (e.g., `T`, `TProps`, `TResult`)
- Type discriminated unions with `type` suffix (e.g., `NodeProfileResponse`)

## Code Style

**Formatting:**
- Tool: Prettier
- Configuration: `ui/.prettierrc`
- Key settings:
  - Semi-colons: enabled
  - Single quotes: true
  - Print width: 100 characters
  - Tab width: 2 spaces
  - Trailing comma: ES5 (objects and arrays)
  - Arrow function parens: avoided when possible (e.g., `x => x * 2`)
  - End of line: LF

**Linting:**
- Tool: ESLint (flat config)
- Configuration: `ui/eslint.config.js`
- Key rules:
  - `no-console`: error, allowing `warn` and `error` methods
  - `@typescript-eslint/no-unused-vars`: errors ignored with `_` prefix
  - `react-refresh/only-export-components`: warn
  - React hooks exhaustive dependencies enforced
  - TypeScript strict mode enabled

## Import Organization

**Order:**
1. External dependencies (React, third-party libraries)
2. Type imports from external packages (`import type {...}`)
3. Relative imports (internal modules, utilities)
4. Relative type imports from internal modules (`import type {...}`)
5. Styles and assets last

**Example pattern from `ui/src/components/dag/DAGChart.tsx`:**
```typescript
import ELK from 'elkjs';
import { useCallback, useEffect, useLayoutEffect, useRef, MouseEvent, type RefObject } from 'react';
import { Background, ReactFlow, /* ... */ type OnMoveStart } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useAtomValue, useSetAtom } from 'jotai';
import type { DAGData } from '@/services/query-plan/types';
import { QueryPlanNode, type QueryPlanNodeData } from '../query-plan/QueryPlanNode';
import { selectedNodeIdsAtom, selectedOperatorLabelAtom } from '@/atoms/dag';
```

**Path Aliases:**
- `@/*`: resolves to `./src/*` for absolute imports
- `~quent/types/*`: resolves to `../examples/simulator/server/ts-bindings/*` for generated type bindings

## Error Handling

**Patterns:**
- Errors thrown as standard `Error` instances with descriptive messages
- API errors include status code and status text: `throw new Error(\`API Error: ${response.status} ${response.statusText}\`)`
- Async functions use try-catch implicitly via Promise rejection
- Component error states handled with conditional rendering
- Error instanceof checks used when type discrimination needed (e.g., `queryBundleError instanceof Error ? queryBundleError.message : 'Unknown error'`)
- Query failures handled with React Query's automatic retry logic (configurable per query)

**Location:** `ui/src/services/api.ts` demonstrates standard error handling

## Logging

**Framework:** `console` (no dedicated logging library)

**Patterns:**
- ESLint configured to allow `console.warn` and `console.error`
- `console.log` prohibited in production code
- Component loading/error states use conditional rendering instead of console logs
- MSW server logs warnings for unhandled requests: `server.listen({ onUnhandledRequest: 'warn' })`

## Comments

**When to Comment:**
- File headers: SPDX license identifier and copyright (NVIDIA 2026)
- Function/hook documentation: JSDoc-style comments explaining purpose
- Complex logic: inline comments explaining why (not what)
- TODOs: marked as `// TODO:` with context about next steps
- Example: `// TODO: Currently fetching root plan when bundle loads - is this correct?`

**JSDoc/TSDoc:**
- Used for exported functions and hooks
- Documents parameters, return types, and potential errors
- Example from `ui/src/services/api.ts`:
```typescript
/**
 * Generic API fetch helper
 * @param endpoint - API endpoint to call
 * @param options - Optional params and fetch options
 */
export async function apiFetch<T>(endpoint: string, options?: ApiFetchOptions): Promise<T>
```

- Test utilities include JSDoc explaining purpose and any caveats
- Used for type definitions when behavior is non-obvious

## Function Design

**Size:**
- Functions kept concise and focused on single responsibility
- React hooks typically 30-100 lines
- Utility functions under 50 lines when possible
- Lazy imports used for large dependencies (e.g., ELK/elkjs is lazy-loaded into DAGChart)

**Parameters:**
- Named parameters used via object destructuring when multiple params needed
- Interfaces defined for parameter objects: `interface QueryBundleParams { engineId: string; queryId: string; }`
- Generic type parameters used for reusable utilities: `function apiFetch<T>(...)`
- React component props typed as `interface ComponentNameProps`

**Return Values:**
- Async functions return typed Promises
- Hooks return values or state tuples following React patterns
- Query factories return `queryOptions` objects for TanStack Router integration
- Utility functions return appropriate types with TypeScript inference

## Module Design

**Exports:**
- Named exports for functions and types (not default exports)
- Query option factories exported for route loader pre-population
- Custom hooks exported for use across components
- Barrel files (index.ts) rarely used; imports directly from source files

**Type Organization:**
- Types kept near their usage
- Shared types imported from `~quent/types/*` (generated bindings)
- Component-local types defined in same file when not shared
- Global types defined in `ui/src/types.ts` with re-exports from generated types

## File Header Convention

**All source files begin with:**
```
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
```

This appears consistently across:
- `ui/src/**/*.ts`, `ui/src/**/*.tsx`
- All Rust source files in `crates/`
- Test setup files

---

*Convention analysis: 2026-03-25*
