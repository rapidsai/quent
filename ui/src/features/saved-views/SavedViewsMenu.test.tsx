// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useLayoutEffect } from 'react';
import { Provider as JotaiProvider } from 'jotai';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useSetDebouncedZoomRange, useSetZoomRange } from '@quent/hooks';
import { render, screen, userEvent, waitFor } from '@/test/test-utils';
import { DeepLinkBoundary } from '@/features/deep-link';
import { decodeDeepLinkState, encodeDeepLinkState } from '@/features/deep-link/deepLink.codec';
import { readSavedViews, savedViewsStorageKey, writeSavedViews } from './savedViews.storage';
import { SavedViewsMenu } from './SavedViewsMenu';

const scope = { engineId: 'engine-a', queryId: 'query-a' };

function SeedViewport() {
  const setZoom = useSetZoomRange();
  const setDebouncedZoom = useSetDebouncedZoomRange();
  useLayoutEffect(() => {
    setZoom({ start: 10, end: 20 });
    setDebouncedZoom({ start: 10, end: 20 });
  }, [setDebouncedZoom, setZoom]);
  return (
    <button
      type="button"
      onClick={() => {
        setZoom({ start: 30, end: 40 });
        setDebouncedZoom({ start: 30, end: 40 });
      }}
    >
      Modify current view
    </button>
  );
}

function renderMenu(onLoadSavedView = vi.fn()) {
  render(
    <JotaiProvider>
      <DeepLinkBoundary
        {...scope}
        activeTab="timeline"
        initialPlanId=""
        durationSeconds={100}
        operators={[]}
        isQueryReady
        onLoadSavedView={onLoadSavedView}
      >
        <SeedViewport />
        <SavedViewsMenu {...scope} activeTab="timeline" />
      </DeepLinkBoundary>
    </JotaiProvider>
  );
  return onLoadSavedView;
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('SavedViewsMenu', () => {
  it('creates a view with an editable generated name and loads it', async () => {
    const onLoad = renderMenu();
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    expect(screen.queryByRole('textbox', { name: 'Saved view name' })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Save current view' }));
    const name = screen.getByRole('textbox', { name: 'Saved view name' });
    expect((name as HTMLInputElement).value).toMatch(/^Timeline · /u);

    await userEvent.clear(name);
    await userEvent.type(name, 'Investigation');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(screen.getByRole('button', { name: 'Saved views' })).toHaveAttribute(
      'aria-expanded',
      'false'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Load Investigation (Timeline)' })).toBeVisible()
    );
    const stored = readSavedViews(localStorage, scope);
    expect(stored).toMatchObject({
      ok: true,
      value: { views: [{ name: 'Investigation', tab: 'timeline' }] },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Load Investigation (Timeline)' }));
    expect(onLoad).toHaveBeenCalledOnce();
    expect(onLoad).toHaveBeenCalledWith(expect.objectContaining({ name: 'Investigation' }));
  });

  it('updates an existing view while preserving its identity', async () => {
    let now = 100;
    vi.spyOn(Date, 'now').mockImplementation(() => now);
    renderMenu();
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save current view' }));
    const name = screen.getByRole('textbox', { name: 'Saved view name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Investigation');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));

    const stored = readSavedViews(localStorage, scope);
    if (!stored.ok) {
      throw new Error(stored.message);
    }
    const original = stored.value.views[0];
    expect(
      writeSavedViews(localStorage, scope, {
        views: [{ ...original, extras: { pinnedCollapsed: true } }],
        invalidEntries: [],
      })
    ).toEqual({ ok: true });

    now = 200;
    await userEvent.click(screen.getByRole('button', { name: 'Modify current view' }));
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    const row = screen.getByRole('button', { name: 'Load Investigation (Timeline)' }).parentElement;
    expect(row?.querySelectorAll('button')).toHaveLength(4);
    expect(
      [...(row?.querySelectorAll('button') ?? [])].map(button => button.getAttribute('aria-label'))
    ).toEqual([
      'Load Investigation (Timeline)',
      'Rename Investigation',
      'Update Investigation',
      'Delete Investigation',
    ]);
    await userEvent.click(screen.getByRole('button', { name: 'Update Investigation' }));
    expect(screen.getByRole('button', { name: 'Saved views' })).toHaveAttribute(
      'aria-expanded',
      'false'
    );

    const updated = readSavedViews(localStorage, scope);
    expect(updated).toMatchObject({
      ok: true,
      value: {
        views: [
          {
            id: original.id,
            name: 'Investigation',
            createdAt: original.createdAt,
            updatedAt: 200,
          },
        ],
      },
    });
    if (!updated.ok) {
      throw new Error(updated.message);
    }
    expect(updated.value.views).toHaveLength(1);
    expect(updated.value.views[0].extras).toBeUndefined();
    expect(decodeDeepLinkState(updated.value.views[0].payload)).toMatchObject({
      ok: true,
      value: {
        data: {
          timeline: { zoomRange: { start: 30, end: 40 } },
        },
      },
    });
  });

  it('renames and confirms deletion of a saved view', async () => {
    renderMenu();
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save current view' }));
    const name = screen.getByRole('textbox', { name: 'Saved view name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Original');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));

    await userEvent.click(await screen.findByRole('button', { name: 'Rename Original' }));
    const rename = screen.getByRole('textbox', { name: 'Rename Original' });
    await userEvent.clear(rename);
    await userEvent.type(rename, 'Renamed{Enter}');
    expect(await screen.findByRole('button', { name: 'Load Renamed (Timeline)' })).toBeVisible();

    await userEvent.click(screen.getByRole('button', { name: 'Delete Renamed' }));
    expect(screen.getByText('Delete “Renamed”?')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Delete' }));
    expect(screen.getByText('No saved views for this query.')).toBeVisible();
  });

  it('cancels creating a saved view without closing the menu', async () => {
    renderMenu();
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save current view' }));
    expect(screen.getByRole('textbox', { name: 'Saved view name' })).toBeVisible();

    await userEvent.click(screen.getByRole('button', { name: 'Cancel saving view' }));

    expect(screen.queryByRole('textbox', { name: 'Saved view name' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save current view' })).toHaveFocus();
    expect(screen.getByRole('button', { name: 'Saved views' })).toHaveAttribute(
      'aria-expanded',
      'true'
    );
  });

  it('returns to the saved-view list when escape is pressed while naming a view', async () => {
    renderMenu();
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save current view' }));
    await userEvent.tab();

    await userEvent.keyboard('{Escape}');

    expect(screen.queryByRole('textbox', { name: 'Saved view name' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save current view' })).toHaveFocus();
    expect(screen.getByRole('button', { name: 'Saved views' })).toHaveAttribute(
      'aria-expanded',
      'true'
    );
  });

  it('exits save mode before renaming an existing view', async () => {
    renderMenu();
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save current view' }));
    const name = screen.getByRole('textbox', { name: 'Saved view name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Investigation');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));

    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save current view' }));
    await userEvent.click(screen.getByRole('button', { name: 'Rename Investigation' }));

    expect(screen.queryByRole('textbox', { name: 'Saved view name' })).not.toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Rename Investigation' })).toBeVisible();
  });

  it('refreshes when another browser tab changes the query scope', async () => {
    renderMenu();
    await userEvent.click(screen.getByRole('button', { name: 'Saved views' }));
    expect(screen.getByText('No saved views for this query.')).toBeVisible();
    const payload = encodeDeepLinkState({
      route: { ...scope, tab: 'timeline' },
      timeline: { zoomRange: { start: 10, end: 20 } },
    });
    if (!payload.ok) {
      throw new Error(payload.message);
    }
    expect(
      writeSavedViews(localStorage, scope, {
        views: [
          {
            id: '99a8834d-768a-4eb4-9b72-1e7987f8cbda',
            name: 'From another tab',
            createdAt: 1,
            updatedAt: 1,
            tab: 'timeline',
            payload: payload.value,
          },
        ],
        invalidEntries: [],
      })
    ).toEqual({ ok: true });

    window.dispatchEvent(
      new StorageEvent('storage', {
        key: savedViewsStorageKey(scope),
        storageArea: localStorage,
      })
    );
    expect(
      await screen.findByRole('button', { name: 'Load From another tab (Timeline)' })
    ).toBeVisible();
  });
});
