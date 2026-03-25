// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable react-refresh/only-export-components */
import { ReactNode } from 'react';
import { render, RenderOptions, renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider, createMemoryHistory, createRouter } from '@tanstack/react-router';
import { routeTree } from '@/routeTree.gen';

/**
 * Create a fresh QueryClient for each test to prevent state leakage
 */
function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false, // Don't retry failed queries in tests
        gcTime: 0, // Garbage collect immediately
        staleTime: 0, // Always consider data stale
      },
      mutations: {
        retry: false,
      },
    },
  });
}

/**
 * Wrapper component that provides QueryClient context
 */
export function QueryWrapper({ children }: { children: ReactNode }) {
  const queryClient = createTestQueryClient();
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}

/**
 * Render a component with QueryClient provider
 */
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

/**
 * Render a hook with QueryClient provider
 */
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

interface RenderWithRouterOptions extends Omit<RenderOptions, 'wrapper'> {
  initialPath?: string;
}

/**
 * Render the app with Router and QueryClient providers
 * Useful for testing route components
 */
export function renderWithRouter(options: RenderWithRouterOptions = {}) {
  const { initialPath = '/', ...renderOptions } = options;
  const queryClient = createTestQueryClient();
  const memoryHistory = createMemoryHistory({ initialEntries: [initialPath] });

  const router = createRouter({
    routeTree,
    history: memoryHistory,
  });

  // Need to cast due to TanStack Router's strict typing
  const typedRouter = router as typeof router;

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

// Re-export everything from testing-library
export * from '@testing-library/react';
export { default as userEvent } from '@testing-library/user-event';
