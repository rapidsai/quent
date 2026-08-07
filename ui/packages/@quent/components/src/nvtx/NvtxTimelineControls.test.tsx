// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { NvtxTimelineControls } from './NvtxTimelineControls';

describe('NvtxTimelineControls', () => {
  it('renders hierarchical filters and independently toggles Uncategorized', async () => {
    const onSelectionChange = vi.fn();
    const user = userEvent.setup();
    render(
      <NvtxTimelineControls
        catalogs={[
          {
            contextId: 'context-1',
            label: 'context context-',
            catalog: {
              trace_start: 0,
              trace_end: 10,
              anomalies: {
                orphan_range_ends: '0',
                orphan_range_pops: '0',
                orphan_resource_destroys: '0',
                reused_range_ids: '0',
                reused_resource_handles: '0',
                total: '0',
                is_faithful: true,
              },
              domains: [
                {
                  domain_id: '9',
                  name: 'Runtime domain with a long display name',
                  color: '#2563eb',
                  threads: [],
                  categories: [{ category_id: 3, name: 'Compute' }],
                  has_uncategorized: true,
                },
              ],
            },
          },
        ]}
        selections={{
          'context-1': [{ domain_id: '9', category_ids: [3], include_uncategorized: true }],
        }}
        statistics={[
          {
            contextId: 'context-1',
            label: 'context context-',
            statistics: [],
          },
        ]}
        onSelectionChange={onSelectionChange}
      />
    );
    await user.click(screen.getByRole('button', { name: /filter nvtx lanes/i }));
    expect(screen.getByText('Runtime domain with a long display name')).toBeInTheDocument();
    await user.click(screen.getByText('Runtime domain with a long display name'));
    await user.click(screen.getByRole('checkbox', { name: 'Uncategorized' }));
    expect(onSelectionChange).toHaveBeenCalledWith('context-1', [
      { domain_id: '9', category_ids: [3], include_uncategorized: false },
    ]);
  });

  it('renders bigint counts and relative-second durations', async () => {
    const user = userEvent.setup();
    render(
      <NvtxTimelineControls
        catalogs={[]}
        selections={{}}
        statistics={[
          {
            contextId: 'context-1',
            label: 'context context-',
            statistics: [
              {
                message: 'long work',
                domain_id: '9',
                domain_name: 'Runtime',
                category_id: null,
                category_name: null,
                count: 2n,
                observed_count: 1n,
                total_duration: 1.25,
                avg_duration: 1.25,
                min_duration: 0.001,
                max_duration: 1.25,
                saturated: false,
              },
            ],
          },
        ]}
        onSelectionChange={vi.fn()}
      />
    );

    await user.click(screen.getByRole('button', { name: 'NVTX statistics' }));
    expect(screen.getAllByText('1.25s')).toHaveLength(3);
    expect(screen.getByText('1.00ms')).toBeInTheDocument();
    expect(screen.getByText(/includes incomplete observations/i)).toBeInTheDocument();
  });
});
