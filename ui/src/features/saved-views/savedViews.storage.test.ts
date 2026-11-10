// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it } from 'vitest';
import {
  MAX_SAVED_VIEWS_PER_QUERY,
  SAVED_VIEWS_SCHEMA,
  SAVED_VIEWS_VERSION,
  type SavedViewV1,
} from './savedViews.schema';
import {
  readSavedViews,
  resetSavedViews,
  savedViewsStorageKey,
  writeSavedViews,
  type SavedViewsScope,
} from './savedViews.storage';

const scope: SavedViewsScope = { engineId: 'engine/a', queryId: 'query:b' };

function view(overrides: Partial<SavedViewV1> = {}): SavedViewV1 {
  return {
    id: '99a8834d-768a-4eb4-9b72-1e7987f8cbda',
    name: 'Timeline view',
    createdAt: 1_700_000_000_000,
    updatedAt: 1_700_000_000_000,
    tab: 'timeline',
    payload: 'v3.abc_123',
    ...overrides,
  };
}

beforeEach(() => {
  localStorage.clear();
});

describe('savedViewsStorageKey', () => {
  it('isolates and encodes engine and query identifiers', () => {
    expect(savedViewsStorageKey(scope)).toBe('quent.savedViews.v1:engine%2Fa:query%3Ab');
    expect(savedViewsStorageKey({ ...scope, queryId: 'other' })).not.toBe(
      savedViewsStorageKey(scope)
    );
  });
});

describe('saved view storage', () => {
  it('round trips valid views', () => {
    expect(writeSavedViews(localStorage, scope, { views: [view()], invalidEntries: [] })).toEqual({
      ok: true,
    });

    expect(readSavedViews(localStorage, scope)).toEqual({
      ok: true,
      value: { views: [view()], invalidEntries: [] },
    });
  });

  it('keeps invalid entries when valid views are rewritten', () => {
    const invalidEntry = { name: 'legacy unknown data' };
    localStorage.setItem(
      savedViewsStorageKey(scope),
      JSON.stringify({
        schema: SAVED_VIEWS_SCHEMA,
        version: SAVED_VIEWS_VERSION,
        ...scope,
        views: [view(), invalidEntry],
      })
    );

    const read = readSavedViews(localStorage, scope);
    expect(read).toEqual({
      ok: true,
      value: { views: [view()], invalidEntries: [invalidEntry] },
    });
    if (!read.ok) {
      throw new Error('Expected saved views to be readable');
    }
    expect(
      writeSavedViews(localStorage, scope, {
        views: [view({ name: 'Renamed' })],
        invalidEntries: read.value.invalidEntries,
      })
    ).toEqual({ ok: true });
    expect(JSON.parse(localStorage.getItem(savedViewsStorageKey(scope)) ?? '')).toMatchObject({
      views: [view({ name: 'Renamed' }), invalidEntry],
    });
  });

  it('reports corrupt JSON without overwriting it', () => {
    localStorage.setItem(savedViewsStorageKey(scope), '{not-json');

    expect(readSavedViews(localStorage, scope)).toMatchObject({
      ok: false,
      kind: 'corrupt',
    });
    expect(localStorage.getItem(savedViewsStorageKey(scope))).toBe('{not-json');
  });

  it('treats future versions as read-only', () => {
    localStorage.setItem(
      savedViewsStorageKey(scope),
      JSON.stringify({
        schema: SAVED_VIEWS_SCHEMA,
        version: SAVED_VIEWS_VERSION + 1,
        ...scope,
        views: [],
      })
    );

    expect(readSavedViews(localStorage, scope)).toMatchObject({
      ok: false,
      kind: 'future-version',
    });
  });

  it('reports older document versions distinctly from scope mismatches', () => {
    localStorage.setItem(
      savedViewsStorageKey(scope),
      JSON.stringify({
        schema: SAVED_VIEWS_SCHEMA,
        version: SAVED_VIEWS_VERSION - 1,
        ...scope,
        views: [],
      })
    );

    expect(readSavedViews(localStorage, scope)).toMatchObject({
      ok: false,
      kind: 'corrupt',
      message: expect.stringContaining('older unsupported format'),
    });
  });

  it('reports unavailable storage and quota errors', () => {
    const unavailable = {
      getItem: () => {
        throw new DOMException('blocked', 'SecurityError');
      },
    } as unknown as Storage;
    expect(readSavedViews(unavailable, scope)).toMatchObject({
      ok: false,
      kind: 'unavailable',
    });

    const quota = {
      setItem: () => {
        throw new DOMException('full', 'QuotaExceededError');
      },
    } as unknown as Storage;
    expect(writeSavedViews(quota, scope, { views: [view()], invalidEntries: [] })).toMatchObject({
      ok: false,
      kind: 'quota',
    });
  });

  it('enforces the per-query view limit', () => {
    const views = Array.from({ length: MAX_SAVED_VIEWS_PER_QUERY + 1 }, (_, index) =>
      view({
        id: `99a8834d-768a-4eb4-9b72-${String(index).padStart(12, '0')}`,
      })
    );
    expect(writeSavedViews(localStorage, scope, { views, invalidEntries: [] })).toMatchObject({
      ok: false,
      kind: 'limit',
    });
    expect(localStorage.getItem(savedViewsStorageKey(scope))).toBeNull();
  });

  it('resets only the current scope', () => {
    localStorage.setItem(savedViewsStorageKey(scope), 'current');
    const otherScope = { ...scope, queryId: 'other' };
    localStorage.setItem(savedViewsStorageKey(otherScope), 'other');

    expect(resetSavedViews(localStorage, scope)).toEqual({ ok: true });
    expect(localStorage.getItem(savedViewsStorageKey(scope))).toBeNull();
    expect(localStorage.getItem(savedViewsStorageKey(otherScope))).toBe('other');
  });
});
