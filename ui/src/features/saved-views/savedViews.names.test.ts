// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { defaultSavedViewName } from './savedViews.names';

describe('defaultSavedViewName', () => {
  it('includes the active tab and formatted date', () => {
    expect(defaultSavedViewName('operators', new Date(2026, 10, 10, 15, 55), [])).toMatch(
      /^Operators · Nov 10, 3:55 PM$/u
    );
  });

  it('adds a case-insensitive collision suffix', () => {
    const now = new Date(2026, 10, 10, 15, 55);
    const first = defaultSavedViewName('timeline', now, []);

    expect(defaultSavedViewName('timeline', now, [first.toLocaleLowerCase(), `${first} (2)`])).toBe(
      `${first} (3)`
    );
  });
});
