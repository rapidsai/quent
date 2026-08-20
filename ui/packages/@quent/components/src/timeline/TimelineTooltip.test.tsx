// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { EntityTooltipContent, TooltipContent, type ActiveMark } from './TimelineTooltip';
import type { DynamicValue } from '@quent/utils';

// The Rust `DynamicValue` enum serializes externally tagged. This is the shape the
// server actually sends, even though the generated TS type is untagged.
const tagged = (v: object) => v as unknown as DynamicValue;

describe('TooltipContent active marks', () => {
  const series = [{ color: '#8884d8', name: 'computing', value: 1 }];
  const makeMarks = (count: number): ActiveMark[] =>
    Array.from({ length: count }, (_, index) => ({
      label: `task-${index}`,
      stateName: 'computing',
      color: '#ff0000',
      durationMs: 500,
    }));

  const renderWithMarks = (marks: ActiveMark[]) =>
    render(<TooltipContent timestamp={3360} series={series} windowMs={5300} activeMarks={marks} />);

  it('renders attribute rows with byte and rate formatting', () => {
    renderWithMarks([
      {
        label: 'task',
        stateName: 'computing',
        color: '#ff0000',
        durationMs: 750,
        attributes: [
          { key: 'input_bytes', value: tagged({ U64: 1_500_000_000 }) },
          { key: 'current_operator_id', value: tagged({ U32: 11 }) },
        ],
        derivedAttributes: [{ key: 'bytes_per_sec', value: tagged({ F64: 2_000_000_000 }) }],
      },
    ]);

    expect(screen.getByText('input_bytes')).toBeInTheDocument();
    expect(screen.getByText('1.40 GiB')).toBeInTheDocument();
    expect(screen.getByText('current_operator_id')).toBeInTheDocument();
    expect(screen.getByText('11')).toBeInTheDocument();
    expect(screen.getByText('duration')).toBeInTheDocument();
    expect(screen.getByText('750.00ms')).toBeInTheDocument();
    expect(screen.getByText('derived')).toBeInTheDocument();
    expect(screen.getByText('bytes_per_sec')).toBeInTheDocument();
    expect(screen.getByText('2.00 GB/s')).toBeInTheDocument();
  });

  it('wraps long string attributes (e.g. a synthesized pipeline chain)', () => {
    renderWithMarks([
      {
        label: 'task-21',
        stateName: 'computing',
        color: '#ff0000',
        durationMs: 26,
        derivedAttributes: [
          {
            key: 'pipeline',
            value: tagged({ String: 'GPU_SCAN(11) -> PROJECTION(6) -> HASH_GROUP_BY(8)' }),
          },
        ],
      },
    ]);

    expect(screen.getByText('pipeline')).toBeInTheDocument();
    expect(
      screen.getByText('GPU_SCAN(11) -> PROJECTION(6) -> HASH_GROUP_BY(8)')
    ).toBeInTheDocument();
  });

  it('renders marks without attributes as before', () => {
    renderWithMarks([{ label: 'task-0', stateName: 'sending', color: '#0000ff' }]);
    expect(screen.getByText('task-0')).toBeInTheDocument();
    expect(screen.getByText('sending')).toBeInTheDocument();
  });

  it('renders entity-only content without a timeline total', () => {
    render(
      <EntityTooltipContent
        timestamp={1_000}
        windowMs={5_000}
        activeMarks={[
          {
            label: 'task-0',
            stateName: 'loading',
            color: '#0000ff',
            durationMs: 500,
          },
        ]}
      />
    );
    expect(screen.getByText('task-0')).toBeInTheDocument();
    expect(screen.getByText('loading')).toBeInTheDocument();
    expect(screen.queryByText('Total')).not.toBeInTheDocument();
  });

  it('keeps full details for six or fewer overlapping entities', () => {
    render(<EntityTooltipContent timestamp={1_000} windowMs={5_000} activeMarks={makeMarks(6)} />);

    expect(screen.getAllByText('500.00ms')).toHaveLength(6);
  });

  it('caps detailed rows and reports entities not shown', () => {
    render(<EntityTooltipContent timestamp={1_000} windowMs={5_000} activeMarks={makeMarks(7)} />);

    expect(screen.getByText('task-0')).toBeInTheDocument();
    expect(screen.getByText('task-5')).toBeInTheDocument();
    expect(screen.queryByText('task-6')).not.toBeInTheDocument();
    expect(screen.getAllByText('500.00ms')).toHaveLength(6);
    expect(screen.getByText('1 more entity not shown')).toBeInTheDocument();
  });
});
