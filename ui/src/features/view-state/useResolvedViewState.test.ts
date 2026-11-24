// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { EncodedViewStateSource } from './viewState.storage';
import { useResolvedViewState } from './useResolvedViewState';

const SCOPE = { engineId: 'e', queryId: 'q', tab: 'timeline' as const };

function renderResolver(sources: readonly EncodedViewStateSource[]) {
  return renderHook(() =>
    useResolvedViewState({
      enabled: true,
      resolutionKey: 'e:q',
      scope: SCOPE,
      sources,
    })
  );
}

describe('useResolvedViewState', () => {
  it('uses the first source containing state', async () => {
    const fallbackLoad = vi.fn(() => 'v3.fallback');
    const { result } = renderResolver([
      { id: 'url', load: () => 'v3.url' },
      { id: 'database', load: fallbackLoad },
    ]);

    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current).toEqual({
      status: 'ready',
      encodedState: 'v3.url',
      sourceId: 'url',
    });
    expect(fallbackLoad).not.toHaveBeenCalled();
  });

  it('supports asynchronous repositories', async () => {
    const { result } = renderResolver([
      { id: 'url', load: () => null },
      { id: 'database', load: async () => 'v3.database' },
    ]);

    await waitFor(() =>
      expect(result.current).toEqual({
        status: 'ready',
        encodedState: 'v3.database',
        sourceId: 'database',
      })
    );
  });

  it('surfaces source failures instead of silently changing precedence', async () => {
    const fallbackLoad = vi.fn(() => 'v3.fallback');
    const error = new Error('database unavailable');
    const { result } = renderResolver([
      {
        id: 'database',
        load: async () => {
          throw error;
        },
      },
      { id: 'local-storage', load: fallbackLoad },
    ]);

    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current).toEqual({
      status: 'error',
      encodedState: null,
      sourceId: 'database',
      error,
    });
    expect(fallbackLoad).not.toHaveBeenCalled();
  });
});
