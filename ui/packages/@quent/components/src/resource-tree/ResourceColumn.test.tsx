// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ResourceColumn } from './ResourceColumn';
import type { TreeTableItem } from './types';

const resourceItem = {
  id: 'res-1',
  type: 'Resource',
  entity: { id: 'res-1', instance_name: 'Storage', type_name: 'storage' },
  children: [],
} as unknown as TreeTableItem;

const groupItem = {
  id: 'group-1',
  type: 'ResourceGroup',
  entity: { id: 'group-1', instance_name: 'task-executor', type_name: 'TaskExecutor' },
  children: [resourceItem],
} as unknown as TreeTableItem;

function renderColumn(item: TreeTableItem, description?: React.ReactNode) {
  render(
    <ResourceColumn
      item={item}
      selectedType="storage"
      onTypeChange={vi.fn()}
      description={description}
    />
  );
}

describe('ResourceColumn description', () => {
  it('shows no description by default', () => {
    renderColumn(resourceItem);

    expect(screen.queryByText('Worker: worker-0')).not.toBeInTheDocument();
  });

  it('shows the description under a resource', () => {
    renderColumn(resourceItem, 'Worker: worker-0');

    expect(screen.getByText('Worker: worker-0')).toBeInTheDocument();
  });

  it('puts the description in the same column as the name and type so the text lines up', () => {
    renderColumn(resourceItem, 'Worker: worker-0');

    const nameColumn = screen.getByText('Storage').closest('div')?.parentElement;
    expect(nameColumn).toContainElement(screen.getByText('Worker: worker-0'));
    expect(screen.getByText('Worker: worker-0').parentElement).toBe(nameColumn);
  });

  it('shows the description under a resource group', () => {
    renderColumn(groupItem, 'Worker: worker-1');

    expect(screen.getByText('Worker: worker-1')).toBeInTheDocument();
  });
});
