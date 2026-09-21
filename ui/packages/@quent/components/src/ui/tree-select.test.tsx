// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { TreeSelect } from './tree-select';
import type { TreeDataItem } from './tree-view';

interface TestItem extends TreeDataItem {
  detail: string;
  children?: TestItem[];
}

const data: TestItem[] = [
  {
    id: 'root',
    name: 'Root plan',
    detail: 'Coordinator',
    children: [
      {
        id: 'child',
        name: 'Child plan',
        detail: 'Worker 1',
      },
    ],
  },
];

describe('TreeSelect', () => {
  it('renders the selected value and selects an item from the tree', async () => {
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <TreeSelect
        data={data}
        value="root"
        ariaLabel="Query plan"
        title="Query Plan"
        getItemTitle={item => `${item.name} — ${item.detail}`}
        onValueChange={onValueChange}
        collapsible={false}
        renderValue={item => (
          <>
            <span>{item.name}</span>
            <span>{item.detail}</span>
          </>
        )}
        renderItem={({ item }) => (
          <>
            <span>{item.name}</span>
            <span>{item.detail}</span>
          </>
        )}
      />
    );

    const trigger = screen.getByRole('combobox', { name: 'Query plan' });
    expect(trigger).toHaveAttribute('title', 'Root plan — Coordinator');
    expect(trigger).toHaveTextContent('Root plan');
    expect(trigger).toHaveTextContent('Coordinator');

    await user.click(trigger);
    expect(screen.getByText('Child plan')).toBeVisible();
    expect(screen.getByText('Child plan').closest('[title]')).toHaveAttribute(
      'title',
      'Child plan — Worker 1'
    );
    expect(screen.queryByRole('button', { name: /Root plan/ })).not.toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Selected item' }).parentElement).toHaveTextContent(
      'Root plan'
    );
    await user.click(screen.getByText('Child plan'));

    expect(onValueChange).toHaveBeenCalledWith(data[0]?.children?.[0]);
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('tree', { name: 'Query plan' })).not.toBeInTheDocument();
  });

  it('closes on outside click and clears the hovered item', async () => {
    const user = userEvent.setup();
    const onItemHover = vi.fn();
    render(
      <>
        <TreeSelect
          data={data}
          value="root"
          ariaLabel="Query plan"
          onValueChange={vi.fn()}
          onItemHover={onItemHover}
        />
        <button type="button">Outside target</button>
      </>
    );

    const trigger = screen.getByRole('combobox', { name: 'Query plan' });
    await user.click(trigger);
    await user.click(screen.getByRole('button', { name: 'Outside target' }));

    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(onItemHover).toHaveBeenLastCalledWith(null);
  });
});
