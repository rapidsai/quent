// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it } from 'vitest';
import { http, HttpResponse } from 'msw';
import { applicationViewStateRepository } from '@/features/view-state';
import { encodeViewState } from '@/features/view-state/viewState.codec';
import type { ViewStateTab } from '@/features/view-state/viewState.schema';
import { server } from '@/test/mocks/server';
import { renderWithRouter, waitFor } from '@/test/test-utils';

const SCOPE = { engineId: 'engine-1', queryId: 'query-1' };
const QUERY_PATH = `/profile/engine/${SCOPE.engineId}/query/${SCOPE.queryId}`;

function storeViewOnTab(tab: ViewStateTab): string {
  const encoded = encodeViewState({
    route: { ...SCOPE, tab },
    timeline: { zoomRange: { start: 0, end: 10 } },
  });
  if (!encoded.ok) {
    throw new Error('fixture failed to encode');
  }
  applicationViewStateRepository.save(SCOPE, encoded.value);
  return encoded.value;
}

describe('query index redirect', () => {
  beforeEach(() => {
    window.localStorage.clear();
    // The redirect happens before data loads, so the tab pages only need to fail quietly.
    server.use(http.get('*/api/*', () => HttpResponse.json({}, { status: 500 })));
  });

  it('reopens the tab the query was last left on', async () => {
    storeViewOnTab('operators');
    const { router } = renderWithRouter({ initialPath: QUERY_PATH });

    await waitFor(() => expect(router.state.location.pathname).toBe(`${QUERY_PATH}/operators`));
  });

  it('opens the timeline when nothing is stored', async () => {
    const { router } = renderWithRouter({ initialPath: QUERY_PATH });

    await waitFor(() => expect(router.state.location.pathname).toBe(`${QUERY_PATH}/timeline`));
  });

  it('ignores the stored tab when opening a shared link', async () => {
    const stored = storeViewOnTab('operators');
    const { router } = renderWithRouter({ initialPath: `${QUERY_PATH}?s=${stored}` });

    await waitFor(() => expect(router.state.location.pathname).toBe(`${QUERY_PATH}/timeline`));
    expect(router.state.location.search).toMatchObject({ s: stored });
  });
});
