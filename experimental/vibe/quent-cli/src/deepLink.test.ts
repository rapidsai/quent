// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  DEEP_LINK_COMPATIBILITY_PATH,
  DEEP_LINK_COMPATIBILITY_PAYLOAD,
  DEEP_LINK_COMPATIBILITY_STATE,
} from './deepLink.compatibility';
import { buildEvidenceDeepLink } from './deepLink';

describe('CLI deep links', () => {
  it('preserves the UI compatibility payload', () => {
    const url = buildEvidenceDeepLink(DEEP_LINK_COMPATIBILITY_PATH, DEEP_LINK_COMPATIBILITY_STATE);

    expect(new URL(url, 'https://quent.example.test').searchParams.get('s')).toBe(
      DEEP_LINK_COMPATIBILITY_PAYLOAD
    );
  });
});
