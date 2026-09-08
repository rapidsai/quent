// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState, type ReactNode } from 'react';
import { Provider as JotaiProvider, createStore } from 'jotai';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { DEFAULT_STALE_TIME, setApiBaseUrl } from '@quent/client';

type JotaiStore = ReturnType<typeof createStore>;

export interface QuentProviderProps {
  children: ReactNode;
  /**
   * Base URL for the Quent API. When provided, applied via {@link setApiBaseUrl}
   * before children render so the first batch of data hooks already sees it.
   * Omit to keep whatever URL was last set globally (e.g. via `VITE_API_BASE_URL`
   * or an earlier `setApiBaseUrl` call).
   */
  apiBaseUrl?: string;
  /**
   * TanStack Query client. When omitted, each provider instance creates its own
   * client with `DEFAULT_STALE_TIME` and `refetchOnWindowFocus: false`. Pass a
   * shared client when multiple providers should share query cache (rare).
   */
  queryClient?: QueryClient;
  /**
   * Jotai store. When omitted, each provider instance gets a fresh isolated
   * store, so multiple sibling provider instances (e.g. several Grafana panels
   * in one dashboard) do not share atom state. Pass an external store to opt
   * into shared state across providers.
   */
  jotaiStore?: JotaiStore;
}

/**
 * One-stop provider that wires up everything `@quent/hooks` and `@quent/components`
 * need at runtime: a `QueryClientProvider`, a `JotaiProvider`, and (optionally)
 * the API base URL.
 *
 * Defaults are tuned for the common embedded case (Grafana panel, Superset
 * chart, isolated visualization in a host app): per-instance `QueryClient` and
 * Jotai store, so dashboards with multiple Quent instances do not cross-talk.
 *
 * ```tsx
 * <QuentProvider apiBaseUrl="https://my-quent-server/api">
 *   <DAGChart data={dag} isDark={isDark} />
 * </QuentProvider>
 * ```
 */
export function QuentProvider({
  children,
  apiBaseUrl,
  queryClient,
  jotaiStore,
}: QuentProviderProps) {
  // Set the API base URL synchronously during render so any data hooks that
  // fire on the first render of `children` already see the configured URL.
  // `setApiBaseUrl` is a cheap idempotent assignment; running it on every
  // render is fine and keeps the URL in sync if the prop changes.
  if (apiBaseUrl !== undefined) {
    setApiBaseUrl(apiBaseUrl);
  }

  // Lazy-init so each provider instance owns its own client/store unless the
  // consumer explicitly passes shared instances. `useState`'s initializer
  // pattern guarantees we don't allocate fresh clients on every render.
  const [defaultQueryClient] = useState<QueryClient>(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: DEFAULT_STALE_TIME,
            refetchOnWindowFocus: false,
          },
        },
      })
  );
  const [defaultJotaiStore] = useState<JotaiStore>(() => createStore());

  const client = queryClient ?? defaultQueryClient;
  const store = jotaiStore ?? defaultJotaiStore;

  return (
    <JotaiProvider store={store}>
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    </JotaiProvider>
  );
}
