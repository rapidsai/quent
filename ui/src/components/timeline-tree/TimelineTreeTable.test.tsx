// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TabActionSlotsContext, type TabActionSlots } from '@/components/TabActionSlots';
import { TimelineTreeTable } from './TimelineTreeTable';

vi.mock('@quent/components', async importOriginal => {
  const actual = await importOriginal<typeof import('@quent/components')>();
  return {
    ...actual,
    TimelineActions: () => <button type="button">Reset zoom</button>,
    TimelineToolbar: ({ filters }: { filters?: ReactNode }) => (
      <div data-testid="standalone-toolbar">{filters}</div>
    ),
  };
});

const slotElements: HTMLElement[] = [];

function createSlot() {
  const slot = document.createElement('div');
  document.body.append(slot);
  slotElements.push(slot);
  return slot;
}

function renderTable(slots: TabActionSlots | null) {
  return render(
    <TabActionSlotsContext.Provider value={slots}>
      <TimelineTreeTable
        durationSeconds={10}
        isDark={false}
        trees={[]}
        controls={{
          expandedIds: new Set(),
          onExpandChange: vi.fn(),
          filters: <input aria-label="Filter resources" />,
        }}
      />
    </TabActionSlotsContext.Provider>
  );
}

describe('TimelineTreeTable controls', () => {
  afterEach(() => {
    slotElements.splice(0).forEach(slot => slot.remove());
  });

  it('renders the filters and actions into the tab bar slots', () => {
    const filters = createSlot();
    const actions = createSlot();
    renderTable({ leftSlot: filters, rightSlot: actions });

    expect(within(filters).getByRole('textbox', { name: 'Filter resources' })).toBeInTheDocument();
    expect(within(actions).getByRole('button', { name: 'Reset zoom' })).toBeInTheDocument();
    expect(screen.queryByTestId('standalone-toolbar')).not.toBeInTheDocument();
  });

  it('renders no controls until the tab bar has mounted its slots', () => {
    renderTable({ leftSlot: null, rightSlot: null });

    expect(screen.queryByRole('textbox', { name: 'Filter resources' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Reset zoom' })).not.toBeInTheDocument();
    expect(screen.queryByTestId('standalone-toolbar')).not.toBeInTheDocument();
  });

  it('draws its own toolbar when no tab bar hosts the controls', () => {
    renderTable(null);

    const toolbar = screen.getByTestId('standalone-toolbar');
    expect(within(toolbar).getByRole('textbox', { name: 'Filter resources' })).toBeInTheDocument();
  });
});
