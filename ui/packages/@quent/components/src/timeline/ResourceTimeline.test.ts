// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { SingleTimelineResponse } from '@quent/utils';
import { resolveOverlayData } from './resourceTimeline.utils';

const CURRENT_DATA = { data: 'current' } as unknown as SingleTimelineResponse;
const RETAINED_DATA = { data: 'retained' } as unknown as SingleTimelineResponse;

describe('resolveOverlayData', () => {
  it('uses current overlay data when available', () => {
    expect(
      resolveOverlayData(
        CURRENT_DATA,
        { cacheKey: 'previous', data: RETAINED_DATA },
        'current',
        true
      )
    ).toBe(CURRENT_DATA);
  });

  it('reuses retained data only for the same operator cache key', () => {
    expect(
      resolveOverlayData(undefined, { cacheKey: 'current', data: RETAINED_DATA }, 'current', true)
    ).toBe(RETAINED_DATA);
    expect(
      resolveOverlayData(undefined, { cacheKey: 'previous', data: RETAINED_DATA }, 'current', true)
    ).toBeUndefined();
  });

  it('returns no overlay without an operator filter', () => {
    expect(resolveOverlayData(CURRENT_DATA, null, 'current', false)).toBeUndefined();
  });
});
