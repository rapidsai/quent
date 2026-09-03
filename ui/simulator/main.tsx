// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider, createHashHistory, createRouter } from '@tanstack/react-router';

import { installSimulatorClient } from '../src/lib/simulatorClient';
import { queryClient } from '../src/lib/queryClient';
import { routeTree } from '../src/routeTree.gen';

import '../src/index.css';

// Static hosts cannot serve arbitrary SPA paths. Hash history keeps copied
// profiler links reloadable without requiring a Pages 404 redirect shim.
const router = createRouter({ routeTree, history: createHashHistory() });

async function start() {
  await installSimulatorClient();

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        {import.meta.env.VITE_DEBUG && !import.meta.env.TEST && (
          <ReactQueryDevtools initialIsOpen={false} />
        )}
      </QueryClientProvider>
    </React.StrictMode>
  );
}

void start().catch(error => {
  const root = document.getElementById('root');
  if (root) {
    root.textContent = `Unable to start simulator demo: ${String(error)}`;
  }
});
