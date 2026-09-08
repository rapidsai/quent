// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { mergeResourceFilter } from './deepLink.cli';

describe('mergeResourceFilter', () => {
  it('preserves stored fields when overriding part of a resource filter', () => {
    const candidate = {
      resources: {
        expandedRowIds: ['resource-a'],
        resourceFilter: {
          search: 'gpu',
          resourceTypes: ['gpu'],
          showOthers: false,
        },
      },
    };

    expect(mergeResourceFilter(candidate, { showOthers: true })).toEqual({
      resources: {
        expandedRowIds: ['resource-a'],
        resourceFilter: {
          search: 'gpu',
          resourceTypes: ['gpu'],
          showOthers: true,
        },
      },
    });
  });

  it('leaves state unchanged when no CLI filter fields are supplied', () => {
    const candidate = { resources: { resourceFilter: { search: 'gpu' } } };

    expect(mergeResourceFilter(candidate, {})).toBe(candidate);
  });
});
