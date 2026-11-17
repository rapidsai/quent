// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { isSavedViewModified } from './savedViews.modified';
import type { SavedViewV1 } from './savedViews.schema';

const view: SavedViewV1 = {
  id: '00000000-0000-4000-8000-000000000001',
  name: 'A',
  createdAt: 1,
  updatedAt: 1,
  tab: 'timeline',
  payload: 'v1.abc',
  extras: { pinnedCollapsed: true, longEntityDensity: 2 },
};

describe('isSavedViewModified', () => {
  it('is false for identical state, ignoring extras key order', () => {
    expect(
      isSavedViewModified(view, {
        ok: true,
        tab: 'timeline',
        payload: 'v1.abc',
        extras: { longEntityDensity: 2, pinnedCollapsed: true },
      })
    ).toBe(false);
  });

  it('is true when payload, tab or extras differ', () => {
    const base = {
      ok: true as const,
      tab: 'timeline' as const,
      payload: 'v1.abc',
      extras: view.extras,
    };
    expect(isSavedViewModified(view, { ...base, payload: 'v1.xyz' })).toBe(true);
    expect(isSavedViewModified(view, { ...base, tab: 'operators' })).toBe(true);
    expect(isSavedViewModified(view, { ...base, extras: undefined })).toBe(true);
  });

  it('treats a failed capture as unmodified', () => {
    expect(isSavedViewModified(view, { ok: false, message: 'x', messages: ['x'] })).toBe(false);
  });
});
