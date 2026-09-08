// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ResourceFilterSearch } from './ResourceFilterSearch';

describe('ResourceFilterSearch', () => {
  it('updates and clears the name and label search', async () => {
    const user = userEvent.setup();
    const onSearchChange = vi.fn();
    const { rerender } = render(
      <ResourceFilterSearch
        fsmTypes={['task']}
        matchCount={2}
        onFsmTypesChange={vi.fn()}
        onResourceTypesChange={vi.fn()}
        onSearchChange={onSearchChange}
        onShowOthersChange={vi.fn()}
        resourceTypes={['gpu']}
        search=""
        selectedFsmTypes={[]}
        selectedResourceTypes={[]}
        showOthers={false}
      />
    );

    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('1 selected filter')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Resource filters' }));
    await user.type(
      screen.getByRole('textbox', { name: 'Search resource and NVTX tree labels' }),
      'GPU'
    );
    expect(onSearchChange).toHaveBeenCalled();

    rerender(
      <ResourceFilterSearch
        fsmTypes={['task']}
        matchCount={1}
        onFsmTypesChange={vi.fn()}
        onResourceTypesChange={vi.fn()}
        onSearchChange={onSearchChange}
        onShowOthersChange={vi.fn()}
        resourceTypes={['gpu']}
        search="GPU"
        selectedFsmTypes={[]}
        selectedResourceTypes={[]}
        showOthers={false}
      />
    );
    expect(
      screen.getByRole('button', { name: 'Resource filters, 1 selected filter' })
    ).toBeInTheDocument();
    expect(screen.getByLabelText('1 selected filter')).toHaveTextContent('1');
    expect(screen.getByText('1 match')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Clear search' }));
    expect(onSearchChange).toHaveBeenLastCalledWith('');
  });

  it('offers separate resource type and FSM type multiselects', async () => {
    const user = userEvent.setup();
    const onResourceTypesChange = vi.fn();
    const onFsmTypesChange = vi.fn();
    const onSearchChange = vi.fn();
    const onShowOthersChange = vi.fn();
    render(
      <ResourceFilterSearch
        fsmTypes={['task', 'transfer']}
        matchCount={0}
        onFsmTypesChange={onFsmTypesChange}
        onResourceTypesChange={onResourceTypesChange}
        onSearchChange={onSearchChange}
        onShowOthersChange={onShowOthersChange}
        resourceTypes={['cpu', 'gpu']}
        search=""
        selectedFsmTypes={[]}
        selectedResourceTypes={[]}
        showOthers={false}
      />
    );

    await user.click(screen.getByRole('button', { name: 'Resource filters' }));
    await user.click(screen.getByRole('combobox', { name: 'Filter by resource type' }));
    const resourceListbox = screen.getByRole('listbox');
    expect(resourceListbox).toHaveAttribute('aria-multiselectable');
    await user.click(screen.getByRole('option', { name: 'gpu' }));
    expect(onResourceTypesChange).toHaveBeenCalledWith(['gpu']);

    await user.click(screen.getByRole('combobox', { name: 'Filter by FSM type' }));
    await user.click(screen.getByRole('option', { name: 'task' }));
    expect(onFsmTypesChange).toHaveBeenCalledWith(['task']);

    const showOthers = screen.getByRole('checkbox', { name: 'Show All' });
    expect(showOthers).not.toBeChecked();
    expect(showOthers).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Clear' })).toBeDisabled();
  });

  it('enables Show All and Clear when filters are active', async () => {
    const user = userEvent.setup();
    const onResourceTypesChange = vi.fn();
    const onFsmTypesChange = vi.fn();
    const onSearchChange = vi.fn();
    const onShowOthersChange = vi.fn();
    render(
      <ResourceFilterSearch
        fsmTypes={['task']}
        matchCount={1}
        onFsmTypesChange={onFsmTypesChange}
        onResourceTypesChange={onResourceTypesChange}
        onSearchChange={onSearchChange}
        onShowOthersChange={onShowOthersChange}
        resourceTypes={['gpu']}
        search="gpu"
        selectedFsmTypes={[]}
        selectedResourceTypes={[]}
        showOthers={false}
      />
    );

    await user.click(screen.getByRole('button', { name: 'Resource filters, 1 selected filter' }));
    const showOthers = screen.getByRole('checkbox', { name: 'Show All' });
    expect(showOthers).toBeEnabled();
    await user.click(showOthers);
    expect(onShowOthersChange).toHaveBeenCalledWith(true);

    const clear = screen.getByRole('button', { name: 'Clear' });
    expect(clear).toBeEnabled();
    await user.click(clear);
    expect(onSearchChange).toHaveBeenLastCalledWith('');
    expect(onResourceTypesChange).toHaveBeenLastCalledWith([]);
    expect(onFsmTypesChange).toHaveBeenLastCalledWith([]);
    expect(onShowOthersChange).toHaveBeenLastCalledWith(false);
  });
});
