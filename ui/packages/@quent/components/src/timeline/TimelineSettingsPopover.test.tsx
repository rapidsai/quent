// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LongEntityDensity } from '@quent/hooks';
import { TimelineSettingsPopover } from './TimelineSettingsPopover';

const mocks = vi.hoisted(() => ({
  density: 3 as LongEntityDensity,
  setDensity: vi.fn(),
}));

vi.mock('@quent/hooks', async importOriginal => ({
  ...(await importOriginal<typeof import('@quent/hooks')>()),
  useLongEntityDensity: () => mocks.density,
  useSetLongEntityDensity: () => mocks.setDensity,
}));

vi.mock('../ui/popover', () => ({
  Popover: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  PopoverTrigger: ({ children }: { children: ReactNode }) => <>{children}</>,
  PopoverContent: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

describe('TimelineSettingsPopover', () => {
  beforeEach(() => {
    mocks.density = 3;
    mocks.setDensity.mockClear();
  });

  it('renders the five entity density snap points', () => {
    render(<TimelineSettingsPopover />);

    const slider = screen.getByRole('slider', { name: 'Entities' });
    expect(slider).toHaveAttribute('min', '1');
    expect(slider).toHaveAttribute('max', '5');
    expect(slider).toHaveAttribute('step', '1');
    expect(slider).toHaveValue('3');
    expect(screen.getByText('Less')).toBeInTheDocument();
    expect(screen.getByText('More')).toBeInTheDocument();
  });

  it('updates the density when the slider moves', () => {
    render(<TimelineSettingsPopover />);

    fireEvent.change(screen.getByRole('slider', { name: 'Entities' }), {
      target: { value: '5' },
    });

    expect(mocks.setDensity).toHaveBeenCalledWith(5);
  });
});
