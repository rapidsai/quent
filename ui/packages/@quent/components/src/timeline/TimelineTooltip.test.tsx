// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TooltipContent, type ActiveMark } from './TimelineTooltip';
import type { Value } from '@quent/utils';

// The Rust `Value` enum serializes externally tagged — this is the shape the
// server actually sends, even though the generated TS type is untagged.
const tagged = (v: object) => v as unknown as Value;

describe('TooltipContent active marks', () => {
  const series = [{ color: '#8884d8', name: 'computing', value: 1 }];

  const renderWithMarks = (marks: ActiveMark[]) =>
    render(
      <TooltipContent
        timestamp={3360}
        series={series}
        startTime={0n}
        windowMs={5300}
        activeMarks={marks}
      />
    );

  it('renders attribute rows with byte formatting and a rate', () => {
    renderWithMarks([
      {
        label: 'task',
        stateName: 'computing',
        color: '#ff0000',
        durationMs: 750,
        processedBytes: 1_500_000_000,
        attributes: [
          { key: 'input_bytes', value: tagged({ U64: 1_500_000_000 }) },
          { key: 'current_operator_id', value: tagged({ U32: 11 }) },
        ],
      },
    ]);

    expect(screen.getByText('input_bytes')).toBeInTheDocument();
    expect(screen.getByText('1.40 GiB')).toBeInTheDocument();
    expect(screen.getByText('current_operator_id')).toBeInTheDocument();
    expect(screen.getByText('11')).toBeInTheDocument();
    expect(screen.getByText('duration')).toBeInTheDocument();
    expect(screen.getByText('750.00ms')).toBeInTheDocument();
    // 1.5 GB over 0.75 s = 2 GB/s (decimal SI).
    expect(screen.getByText('rate')).toBeInTheDocument();
    expect(screen.getByText('2.00 GB/s')).toBeInTheDocument();
  });

  it('renders the resolved operator', () => {
    renderWithMarks([
      {
        label: 'task-21',
        stateName: 'computing',
        color: '#ff0000',
        durationMs: 26,
        operator: {
          name: 'GPU_SCAN(11) -> PROJECTION(6) -> HASH_GROUP_BY(8)',
          typeName: 'Pipeline Id 0',
        },
        attributes: [{ key: 'input_bytes', value: tagged({ U64: 90_956_652 }) }],
      },
    ]);

    expect(screen.getByText('Pipeline Id 0')).toBeInTheDocument();
    expect(
      screen.getByText('GPU_SCAN(11) -> PROJECTION(6) -> HASH_GROUP_BY(8)')
    ).toBeInTheDocument();
  });

  it('omits rate when the mark has no processed bytes', () => {
    renderWithMarks([
      {
        label: 'task',
        stateName: 'allocating',
        color: '#00ff00',
        durationMs: 250,
        attributes: [{ key: 'target_tier', value: tagged({ String: 'GPU' }) }],
      },
    ]);

    expect(screen.getByText('target_tier')).toBeInTheDocument();
    expect(screen.getByText('GPU')).toBeInTheDocument();
    expect(screen.queryByText('rate')).toBeNull();
  });

  it('renders marks without attributes as before', () => {
    renderWithMarks([{ label: 'task-0', stateName: 'sending', color: '#0000ff' }]);
    expect(screen.getByText('task-0')).toBeInTheDocument();
    expect(screen.getByText('sending')).toBeInTheDocument();
    expect(screen.queryByText('rate')).toBeNull();
  });
});
