// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { isLightColor } from './colors';

describe('isLightColor', () => {
  it('distinguishes light and dark six-digit hex colors', () => {
    expect(isLightColor('#ffffff')).toBe(true);
    expect(isLightColor('#000000')).toBe(false);
  });
});
