// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { ResourceUsageList } from './ResourceUsageList';

describe('ResourceUsageList', () => {
  it('renders each resource in a separate box with its capacities below', () => {
    const queryBundle = {
      entities: {
        resources: {
          'gpu-0': {
            id: 'gpu-0',
            instance_name: 'GPU 0',
            type_name: 'Gpu',
            parent_group_id: 'worker-0',
          },
        },
        resource_types: {
          Gpu: {
            name: 'Gpu',
            capacities: [{ name: 'memory', kind: 'Occupancy', quantity: 'bytes' }],
            used_by: [],
          },
        },
      },
      quantity_specs: {
        bytes: {
          symbol: 'B',
          singular: 'byte',
          plural: 'bytes',
          occupancy_prefix: 'Iec',
          rate_prefix: 'Si',
        },
      },
    } as unknown as QueryBundle<EntityRef>;

    render(
      <ResourceUsageList
        usages={[
          {
            resource: 'gpu-0',
            capacities: [
              ['memory', 2048n],
              ['slots', 4n],
              ['unspecified', null],
            ],
          },
          { resource: 'cpu-0', capacities: [] },
        ]}
        resourceLabel={id => (id === 'gpu-0' ? 'GPU 0' : 'CPU 0')}
        queryBundle={queryBundle}
      />
    );

    const usageBoxes = screen.getAllByRole('listitem');
    expect(usageBoxes).toHaveLength(2);

    const gpuUsage = within(usageBoxes[0]!);
    expect(gpuUsage.getByText('GPU 0')).toBeInTheDocument();
    expect(gpuUsage.getByText('memory')).toBeInTheDocument();
    expect(gpuUsage.getByText('2.00 KiB')).toBeInTheDocument();
    expect(gpuUsage.getByText('slots')).toBeInTheDocument();
    expect(gpuUsage.getByText('4')).toBeInTheDocument();
    expect(gpuUsage.getByText('unspecified')).toBeInTheDocument();
    expect(gpuUsage.getByText('—')).toBeInTheDocument();

    expect(within(usageBoxes[1]!).getByText('CPU 0')).toBeInTheDocument();
  });
});
