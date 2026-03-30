// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { http, HttpResponse } from 'msw';

/**
 * Default MSW handlers for mocking API responses
 * Add your API mocks here
 */
export const handlers = [
  // Example: List engines
  http.get('/api/engines', () => {
    return HttpResponse.json(['engine-1', 'engine-2', 'engine-3']);
  }),

  // Example: List coordinators for an engine
  http.get('/api/engines/:engineId/coordinators', ({ params }) => {
    const { engineId } = params;
    return HttpResponse.json([`${engineId}-coordinator-1`, `${engineId}-coordinator-2`]);
  }),

  // Example: List queries
  http.get('/api/engines/:engineId/coordinators/:coordinatorId/queries', () => {
    return HttpResponse.json(['query-1', 'query-2', 'query-3']);
  }),

  // Example: Get query details
  http.get('/api/queries/:queryId', ({ params }) => {
    const { queryId } = params;
    return HttpResponse.json({
      id: queryId,
      status: 'completed',
      createdAt: new Date().toISOString(),
    });
  }),

  // Example: Get node profile data
  http.get('/api/queries/:queryId/nodes/:nodeId/profile', ({ params }) => {
    const { nodeId } = params;
    const timestamps = Array.from({ length: 100 }, (_, i) => Date.now() - i * 1000);
    return HttpResponse.json({
      nodeId,
      timestamps,
      series: {
        CPU: Array.from({ length: 100 }, () => Math.random() * 100),
        Memory: Array.from({ length: 100 }, () => Math.random() * 1000),
        IO: Array.from({ length: 100 }, () => Math.random() * 500),
      },
    });
  }),
];
