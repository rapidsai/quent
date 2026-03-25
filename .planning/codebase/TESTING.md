# Testing Patterns

**Analysis Date:** 2026-03-25

## Test Framework

**Runner:**
- Vitest 4.0.18
- Configuration: `ui/vitest.config.ts`
- Environment: jsdom (browser simulation)
- Globals: enabled (test/describe/it/expect available without imports)

**Assertion Library:**
- Vitest built-in assertions with extended matchers from `@testing-library/jest-dom`

**Run Commands:**
```bash
pnpm test                # Run tests in watch mode
pnpm test:run            # Run tests once (CI mode)
pnpm test:coverage       # Run tests with coverage report
```

**Coverage Settings:**
- Provider: v8
- Reporters: text, json, html, cobertura
- Excluded from coverage:
  - `node_modules/`
  - `src/test/` (test utilities themselves)
  - `src/routeTree.gen.ts` (generated)
  - `**/*.d.ts` (type definitions)
  - `**/*.config.*` (config files)

## Test File Organization

**Location:**
- Co-located with source files using `.test.ts` or `.spec.ts` suffix
- Test utilities in dedicated `ui/src/test/` directory
- Mock setup in `ui/src/test/mocks/`

**Naming:**
- Files: `[module].test.ts` or `[module].spec.ts`
- Example: `api.test.ts` tests the `api.ts` module
- Example: `example.test.tsx` demonstrates setup for React components

**Structure (TypeScript/React files):**
```
ui/src/
├── services/
│   ├── api.ts
│   └── api.test.ts          # Tests co-located with source
├── test/
│   ├── setup.ts             # Global test setup
│   ├── test-utils.tsx       # Render helpers and wrappers
│   ├── mocks/
│   │   ├── handlers.ts      # MSW request handlers
│   │   └── server.ts        # MSW server initialization
│   └── example.test.tsx     # Example test demonstrating setup
```

## Test Structure

**Suite Organization:**
```typescript
import { describe, it, expect } from 'vitest';

describe('Module/Feature Name', () => {
  describe('specific behavior', () => {
    it('should do something specific', () => {
      // Arrange
      const input = 'test';

      // Act
      const result = functionUnderTest(input);

      // Assert
      expect(result).toBe(expected);
    });
  });
});
```

**Patterns:**
- Nested `describe` blocks group related tests
- Each `it` tests a single behavior
- Arrange-Act-Assert pattern (explicit in comments or implied)
- Tests use Testing Library for component/hook testing
- Tests use Vitest assertions for unit logic

**Example from `ui/src/services/api.test.ts`:**
```typescript
describe('parseJsonWithBigInt', () => {
  describe('standard JSON parsing', () => {
    it('should parse simple objects without big integers', () => {
      const json = '{"name": "test", "count": 42}';
      const result = parseJsonWithBigInt<{ name: string; count: number }>(json);
      expect(result).toEqual({ name: 'test', count: 42 });
    });
  });

  describe('BigInt conversion', () => {
    it('should convert integers larger than MAX_SAFE_INTEGER to BigInt', () => {
      const largeInt = '9007199254740993';
      const json = `{"timestamp": ${largeInt}}`;
      const result = parseJsonWithBigInt<{ timestamp: bigint }>(json);
      expect(result.timestamp).toBe(BigInt(largeInt));
      expect(typeof result.timestamp).toBe('bigint');
    });
  });
});
```

## Mocking

**Framework:** Mock Service Worker (MSW) v2.12.10

**Patterns - HTTP Handlers:**
```typescript
// From ui/src/test/mocks/handlers.ts
import { http, HttpResponse } from 'msw';

export const handlers = [
  http.get('/api/engines', () => {
    return HttpResponse.json(['engine-1', 'engine-2', 'engine-3']);
  }),

  http.get('/api/engines/:engineId/coordinators', ({ params }) => {
    const { engineId } = params;
    return HttpResponse.json([`${engineId}-coordinator-1`, `${engineId}-coordinator-2`]);
  }),
];
```

**MSW Server Setup:**
```typescript
// ui/src/test/mocks/server.ts
import { setupServer } from 'msw/node';
import { handlers } from './handlers';

export const server = setupServer(...handlers);
```

**Global Test Setup:**
```typescript
// ui/src/test/setup.ts
import { beforeAll, afterEach, afterAll, vi } from 'vitest';
import { server } from './mocks/server';

beforeAll(() => {
  server.listen({ onUnhandledRequest: 'warn' });
});

afterEach(() => {
  cleanup();
  server.resetHandlers();
});

afterAll(() => {
  server.close();
});
```

**Browser API Mocks:**
- `window.matchMedia`: Mocked for theme-detection components
- `ResizeObserver`: Mocked for Radix UI components
- `Element.prototype.scrollIntoView`: Mocked for Select components

## Fixtures and Factories

**Test Data:**
- No dedicated fixture files for simple test data
- Data created inline in tests with factory functions
- Example from `api.test.ts`:
```typescript
const json = `{
  "span": {
    "start": 1704067200000000000,
    "end": 1704153600000000000
  },
  "uses": [
    {
      "span": {"start": 1704067200000000000, "end": 1704070800000000000},
      "amounts": [{"key": "bytes", "value": {"U64": 1024}}],
      "entity": {"Worker": "worker-1"}
    }
  ]
}`;
```

**QueryClient Test Factory:**
```typescript
// From ui/src/test/test-utils.tsx
function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,           // Don't retry in tests
        gcTime: 0,             // Garbage collect immediately
        staleTime: 0,          // Always consider data stale
      },
      mutations: {
        retry: false,
      },
    },
  });
}
```

## Coverage

**Requirements:** Not enforced via CI check (no coverage threshold in config)

**View Coverage:**
```bash
pnpm test:coverage     # Generates coverage reports in coverage/ directory
```

**Report Formats Generated:**
- `text`: Console output
- `json`: Machine-readable format
- `html`: Interactive HTML coverage report
- `cobertura`: XML format for CI integration

**Current Coverage Gaps:**
- UI components have minimal test coverage (setup demonstrates testing pattern)
- Rust crates do not appear to have unit tests configured

## Test Types

**Unit Tests:**
- Scope: Individual functions/utilities
- Approach: Test function inputs and outputs in isolation
- Examples: `api.test.ts` for `parseJsonWithBigInt` function
- Location: Co-located with source files

**Integration Tests:**
- Scope: Component + hooks + API mocking
- Approach: Use `renderWithQuery` or `renderWithRouter` helpers
- Mocking: MSW mocks HTTP requests, React Query provides cache
- Example pattern (from `example.test.tsx`):
```typescript
it('should have MSW handlers registered', async () => {
  const response = await fetch('/api/engines');
  const data = await response.json();
  expect(response.ok).toBe(true);
  expect(data).toEqual(['engine-1', 'engine-2', 'engine-3']);
});
```

**E2E Tests:**
- Not implemented in current codebase
- Framework available: Playwright integration possible via Vitest

## Common Patterns

**Async Testing:**
```typescript
// From example.test.tsx
it('should have MSW handlers registered', async () => {
  const response = await fetch('/api/engines');
  const data = await response.json();

  expect(response.ok).toBe(true);
  expect(data).toEqual(['engine-1', 'engine-2', 'engine-3']);
});

// Hook async pattern would use waitFor
// import { waitFor } from '@testing-library/react';
```

**Error Testing:**
```typescript
// Pattern for error handling
it('should handle errors', async () => {
  // Override default handler with error response
  server.use(
    http.get('/api/path', () => {
      return HttpResponse.json(
        { error: 'Not found' },
        { status: 404 }
      );
    })
  );

  // Test error handling
  await expect(apiFetch('/api/path')).rejects.toThrow('API Error: 404');
});
```

**Component Testing with Providers:**
```typescript
// From test-utils.tsx
export function renderWithQuery(ui: ReactNode, options?: Omit<RenderOptions, 'wrapper'>) {
  const queryClient = createTestQueryClient();

  function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  }

  return {
    ...render(ui, { wrapper: Wrapper, ...options }),
    queryClient,
  };
}

// Usage in test
it('should render component with QueryClient provider', () => {
  renderWithQuery(<div data-testid="test-element">Hello World</div>);
  expect(screen.getByTestId('test-element')).toBeInTheDocument();
});
```

**Hook Testing:**
```typescript
// Pattern for hook testing with providers
export function renderHookWithQuery<TResult, TProps>(
  hook: (props: TProps) => TResult,
  options?: { initialProps?: TProps }
) {
  const queryClient = createTestQueryClient();

  function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  }

  return {
    ...renderHook(hook, { wrapper: Wrapper, ...options }),
    queryClient,
  };
}
```

**Router Testing:**
```typescript
// From test-utils.tsx
export function renderWithRouter(options: RenderWithRouterOptions = {}) {
  const { initialPath = '/', ...renderOptions } = options;
  const queryClient = createTestQueryClient();
  const memoryHistory = createMemoryHistory({ initialEntries: [initialPath] });

  const router = createRouter({
    routeTree,
    history: memoryHistory,
  });

  return {
    ...render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={typedRouter} />
      </QueryClientProvider>,
      renderOptions
    ),
    queryClient,
    router: typedRouter,
  };
}
```

## What to Mock

**Mock:**
- HTTP requests (via MSW)
- Browser APIs (window.matchMedia, ResizeObserver)
- External dependencies with complex behavior

**Do NOT Mock:**
- React Query (use actual QueryClient with test defaults)
- React Router (use createMemoryHistory + createRouter)
- Testing Library components (render and test real components)
- Standard library functions (unnecessary overhead)

## Test Utilities Export Pattern

`ui/src/test/test-utils.tsx` re-exports Testing Library components:
```typescript
export * from '@testing-library/react';
export { default as userEvent } from '@testing-library/user-event';
```

This allows tests to import everything from a single location:
```typescript
import { screen, renderWithQuery, userEvent } from './test/test-utils';
```

---

*Testing analysis: 2026-03-25*
