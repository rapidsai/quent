// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PositionedTooltip } from './positioned-tooltip';

describe('PositionedTooltip', () => {
  it('portals content beside the pointer', () => {
    render(
      <PositionedTooltip clientX={100} clientY={50}>
        <span>Tooltip content</span>
      </PositionedTooltip>
    );

    const host = screen.getByText('Tooltip content').parentElement;
    expect(host).toHaveStyle({ left: '112px', top: '62px' });
    expect(host).toHaveClass('pointer-events-none', 'fixed', 'z-[1000]');
  });
});
