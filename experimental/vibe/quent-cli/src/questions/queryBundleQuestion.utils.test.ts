// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { renderTable } from './queryBundleQuestion.utils';

describe('query bundle question utilities', () => {
  it('renders an empty table using header widths', () => {
    expect(renderTable(['Name'], [])).toBe(
      ['┌──────┐', '│ Name │', '├──────┤', '└──────┘'].join('\n')
    );
  });

  it('renders tables with more rows than variadic calls support', () => {
    const rows = Array.from({ length: 200_001 }, (_, index) => [
      index === 200_000 ? 'longest' : 'x',
    ]);

    const rendered = renderTable(['Value'], rows);

    expect(rendered).toContain('│ longest │');
  });
});
