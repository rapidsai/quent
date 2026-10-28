// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { QueryDiffResult } from '@quent-experimental/query-diff';
import { formatQueryDiff } from './queryDiffFormat';

describe('query diff terminal formatter', () => {
  it('renders query metadata, metrics, and limitations', () => {
    const result: QueryDiffResult = {
      baseline: {
        engineId: 'engine-1',
        queryId: 'query-1',
        durationSeconds: 10,
      },
      comparisons: [
        {
          candidate: {
            engineId: 'engine-2',
            queryId: 'query-2',
            durationSeconds: 12,
          },
          summary: [
            {
              scope: 'logical',
              metric: 'output_rows',
              quantity: 'rows',
              baseline: '100',
              candidate: '150',
              delta: '50',
              deltaPercent: 50,
            },
            {
              scope: 'physical',
              metric: 'active_span_s',
              quantity: 'seconds',
              baseline: 4,
              candidate: 3,
              delta: -1,
              deltaPercent: -25,
            },
          ],
          rows: [
            {
              scope: 'logical',
              operatorType: 'Scan',
              metric: 'output_rows',
              quantity: 'rows',
              baseline: '100',
              candidate: '150',
              delta: '50',
              deltaPercent: 50,
            },
            {
              scope: 'physical',
              operatorType: 'Filter',
              metric: 'active_span_s',
              quantity: 'seconds',
              baseline: 4,
              candidate: 3,
              delta: -1,
              deltaPercent: -25,
            },
          ],
        },
        {
          candidate: {
            engineId: 'engine-3',
            queryId: 'query-3',
            durationSeconds: 14,
          },
          summary: [],
          rows: [],
        },
      ],
      metrics: ['active_span_s', 'output_rows'],
      limitations: ['Example limitation.'],
    };

    const formatted = formatQueryDiff(result);

    expect(formatted).toContain('Baseline: engine-1 / query-1 (10s)');
    expect(formatted).toContain('Candidates: 2');
    expect(formatted).toContain('Metrics: active_span_s, output_rows');
    expect(formatted).toContain('Candidate 1: engine-2 / query-2 (12s)');
    expect(formatted).toContain('Candidate 2: engine-3 / query-3 (14s)');
    expect(formatted).toContain('Summary · totals across operators by plan type');
    expect(formatted).toContain('│ Plan');
    expect(formatted).toContain('│ Operator');
    expect(formatted).not.toContain('Logical · Scan\n┌');
    expect(formatted).toContain('+50.00%');
    expect(formatted).toContain('- Example limitation.');
    expect(formatted).not.toContain('\u001B[');

    const colored = formatQueryDiff(result, { color: true });
    const redDelta = colored.slice(
      colored.indexOf('\u001B[31m'),
      colored.indexOf('\u001B[39m', colored.indexOf('\u001B[31m'))
    );
    const blueDelta = colored.slice(
      colored.indexOf('\u001B[34m'),
      colored.indexOf('\u001B[39m', colored.indexOf('\u001B[34m'))
    );
    expect(redDelta).toContain('50');
    expect(blueDelta).toContain('-1');

    const grouped = formatQueryDiff(result, { combinedTable: false });
    expect(grouped).toContain('Logical · Scan\n┌');
    expect(grouped).toContain('Physical · Filter\n┌');

    const sourced = formatQueryDiff({
      ...result,
      baseline: { ...result.baseline, source: 'local' },
      comparisons: result.comparisons.map((comparison, index) => ({
        ...comparison,
        candidate: {
          ...comparison.candidate,
          source: index === 0 ? 'db 6647' : 'db 6650',
        },
      })),
    });
    expect(sourced).toContain('Baseline: local / engine-1 / query-1');
    expect(sourced).toContain('Candidate 1: db 6647 / engine-2 / query-2');
  });
});
