// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { isSamePipe } from './pipeRef';

describe('isSamePipe', () => {
  const pipe = { sourcePortId: 'out', targetPortId: 'in' };

  it('matches when both port IDs are equal, ignoring other fields', () => {
    expect(isSamePipe(pipe, { ...pipe, id: 'edge' } as typeof pipe)).toBe(true);
  });

  it('distinguishes parallel pipes that share only one port', () => {
    expect(isSamePipe(pipe, { sourcePortId: 'out', targetPortId: 'other' })).toBe(false);
    expect(isSamePipe(pipe, { sourcePortId: 'other', targetPortId: 'in' })).toBe(false);
  });

  it('never matches a pipe without port IDs', () => {
    expect(isSamePipe({}, {})).toBe(false);
    expect(isSamePipe({ sourcePortId: 'out' }, { sourcePortId: 'out' })).toBe(false);
  });

  it('is false when either side is missing', () => {
    expect(isSamePipe(null, pipe)).toBe(false);
    expect(isSamePipe(pipe, undefined)).toBe(false);
    expect(isSamePipe(null, null)).toBe(false);
  });
});
