# Testing Patterns

**Analysis Date:** 2026-04-01

## Test Framework

**Runner:**
- Vitest 1.0+
- Config: `ui/vitest.config.ts`
- Environment: jsdom for DOM testing

**Assertion Library:**
- @testing-library/jest-dom (extends native expect)
- testing-library/react for component testing
- Custom matchers: `toBeInTheDocument()`, `toHaveClass()`, etc.

**HTTP Mocking:**
- MSW (Mock Service Worker) 2.12.10+
- Node server setup in `src/test/mocks/server.ts`
- Handlers defined in `src/test/mocks/handlers.ts`

**Run Commands:**
```bash
pnpm test                 # Run tests in watch mode
pnpm test:run            # Run all tests once (CI mode)
pnpm test:coverage       # Run tests with coverage report
pnpm ci:check            # Full CI check including tests
```

## Test File Organization

**Location:**
- Co-located with source files: `*.test.ts` or `*.test.tsx` next to source
- Centralized test setup: `src/test/setup.ts`
- Test utilities: `src/test/test-utils.tsx`
- Mock handlers: `src/test/mocks/handlers.ts` and `src/test/mocks/server.ts`

**Naming:**
- Test files: `[FileName].test.ts` or `[FileName].test.tsx`
- Example: `QueryResourceTree.test.tsx`, `api.test.ts`, `profile.index.test.tsx`

**Structure:**
```
src/
├── test/
│   ├── setup.ts           # Global test setup (mocks, configuration)
│   ├── test-utils.tsx     # Custom render functions with providers
│   ├── example.test.tsx   # Example test verifying setup
│   └── mocks/
│       ├── server.ts      # MSW server instance
│       └── handlers.ts    # API request handlers
├── components/
│   ├── QueryResourceTree.tsx
│   └── QueryResourceTree.test.tsx
└── services/
    ├── api.ts
    └── api.test.ts
```

## Test Structure

**Suite Organization:**
```typescript
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { screen, renderWithQuery } from '@/test/test-utils';

describe('ComponentName', () => {
  describe('Feature Group', () => {
    it('should do something specific', () => {
      // Arrange
      const expected = 'value';

      // Act
      const result = doSomething();

      // Assert
      expect(result).toBe(expected);
    });
  });
});
```

**Patterns:**
- Use `describe()` blocks for logical grouping of related tests
- Nested `describe()` for related features (e.g., "Page rendering", "API data fetching")
- Descriptive `it()` test names starting with "should"
- Arrange-Act-Assert pattern

**Example from codebase:**
```typescript
describe('EngineSelectionPage', () => {
  describe('Page rendering', () => {
    it('renders the page title and description', async () => {
      renderWithRouter({ initialPath: '/profile' });
      await waitFor(() => {
        expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
      });
    });
  });
});
```

## Mocking

**Framework:** Vitest `vi` module + MSW

**Global Setup (`src/test/setup.ts`):**
```typescript
// Mock window.matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    // ... event handlers
  })),
});

// Mock ResizeObserver
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver = ResizeObserverMock;

// Mock Element.scrollIntoView
Element.prototype.scrollIntoView = vi.fn();

// Start MSW server
beforeAll(() => server.listen({ onUnhandledRequest: 'warn' }));
afterEach(() => { cleanup(); server.resetHandlers(); });
afterAll(() => server.close());
```

**Component Mocking Pattern:**
```typescript
// Mock hook
vi.mock('@/hooks/useBulkTimelines', () => ({
  useBulkTimelines: () => ({
    handleZoomChange: vi.fn(),
    handleExpand: vi.fn()
  }),
}));

// Mock component
vi.mock('./timeline/TimelineController', () => ({
  TimelineController: (props: Props) => {
    // Capture props for assertion
    capturedData = props.data;
    return null;
  },
}));

// Mock module with selective override
vi.mock('@/services/api', async importOriginal => {
  const actual = await importOriginal<typeof api>();
  return {
    ...actual,
    fetchSingleTimeline: vi.fn()
  };
});
```

**What to Mock:**
- Heavy/visual dependencies (charts, canvas renderers)
- External API calls
- Child components in component tests
- Browser APIs (matchMedia, ResizeObserver, scrollIntoView)
- Global state management when testing in isolation

**What NOT to Mock:**
- User interaction (use userEvent instead)
- React hooks that are part of your component's contract
- Internal utility functions
- Testing library query functions

## Fixtures and Factories

**Test Data Factories:**
```typescript
// Builder function pattern
const makeBundle = (): QueryBundle<EntityRef> =>
  ({
    query_id: 'test-query',
    entities: { /* ... */ },
    resource_tree: { /* ... */ },
    // ...
  }) as unknown as QueryBundle<EntityRef>;

const makeTimeline = (start: number, end: number): SingleTimelineResponse =>
  ({
    config: { span: { start, end }, bin_duration: 1, num_bins: BigInt(end - start) },
    data: { Binned: { series: {} } },
  }) as unknown as SingleTimelineResponse;
```

**Location:**
- Define helpers inside test file for single-file use
- Export from test-utils only if shared across multiple tests

**Constants:**
```typescript
const DURATION_S = 100;
const ROOT_GROUP_ID = 'qg-1';
const RESOURCE_ID = 'res-1';
const RESOURCE_TYPE = 'GPU';
```

## Coverage

**Requirements:** None explicitly enforced

**View Coverage:**
```bash
pnpm test:coverage
```

**Excluded from coverage:**
- `node_modules/`
- `src/test/` (test infrastructure)
- `src/routeTree.gen.ts` (generated)
- `**/*.d.ts` (type definitions)
- `**/*.config.*` (config files)

**Reporters available:** text, json, html, cobertura

## Test Types

**Unit Tests:**
- Scope: Individual functions, utilities, parsers
- Approach: Direct function calls with various inputs
- Example: `parseJsonWithBigInt()` tests with different numeric edge cases

**Component Tests:**
- Scope: React components with their providers
- Approach: Render with testing-library, interact with user events, assert DOM state
- Example: `QueryResourceTree.test.tsx` tests component rendering and state management
- Providers: QueryClient, Jotai store, React Router

**Integration Tests:**
- Scope: Multiple components working together with mocked APIs
- Approach: Render full page sections, mock MSW handlers
- Example: `profile.index.test.tsx` tests page-level interactions with API data

**E2E Tests:**
- Status: Not used in this codebase
- Would require: Playwright, Cypress, or similar browser automation tool

## Common Patterns

**Async Testing:**
```typescript
// Pattern 1: Using waitFor for eventual state
it('should load data', async () => {
  renderWithRouter({ initialPath: '/profile' });

  await waitFor(() => {
    expect(screen.getByText('engine-1')).toBeInTheDocument();
  });
});

// Pattern 2: Using act for state updates
import { act } from '@testing-library/react';
await act(async () => {
  applyBulkTimelineResponse(response, idToMeta, store);
});
```

**Error Testing:**
```typescript
// Mock API to return error
server.use(
  http.get(`${API_BASE}/engines`, () => {
    return new HttpResponse(null, { status: 500 });
  })
);

// Render and verify error handling
renderWithRouter({ initialPath: '/profile' });
await waitFor(() => {
  expect(screen.queryByText('engine-1')).not.toBeInTheDocument();
});
```

**Provider Setup:**
```typescript
// Use custom render functions from test-utils

// With QueryClient only
renderWithQuery(<Component />);

// With Router and QueryClient
renderWithRouter({ initialPath: '/path' });

// With custom store
const store = createStore();
renderWithQuery(
  <JotaiProvider store={store}>
    <Component />
  </JotaiProvider>
);
```

## Custom Test Utilities

**`renderWithQuery()`:** Renders component with QueryClient provider
- Fresh QueryClient per test
- Query retries disabled
- Garbage collection immediate
- Returns render result and queryClient reference

**`renderWithRouter()`:** Renders with Router and QueryClient providers
- Accepts `initialPath` option
- Creates in-memory router history
- Returns render result, queryClient, and router reference
- Used for testing route-dependent components

**`renderHookWithQuery()`:** Renders hooks with QueryClient provider
- Similar setup to renderWithQuery
- For testing custom hooks in isolation

All exports from `@testing-library/react` available, plus `userEvent`

---

*Testing analysis: 2026-04-01*
