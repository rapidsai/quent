// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { nvtxViewportFromZoom } from './nvtxTimeline.utils';

describe('nvtxViewportFromZoom', () => {
  it('passes query-relative timeline seconds to the NVTX viewport contract', () => {
    expect(nvtxViewportFromZoom({ start: -0.25, end: 1.5 })).toEqual({
      start: -0.25,
      end: 1.5,
    });
  });
});
