// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  ACTIVE_SPAN_METRIC,
  type QueryDiffComparison,
  type QueryDiffResult,
  type QueryDiffRow,
  type QueryDiffSummary,
  type SerializedNumericValue,
} from '@quent-experimental/query-diff';

interface QueryDiffFormatOptions {
  color?: boolean;
  combinedTable?: boolean;
}

type DeltaSign = -1 | 0 | 1;

function formatNumeric(value: SerializedNumericValue | null, metric: string): string {
  if (value === null) {
    return '—';
  }
  if (typeof value === 'string') {
    return value;
  }
  if (metric === ACTIVE_SPAN_METRIC) {
    return value.toFixed(6).replace(/\.?0+$/u, '');
  }
  return value.toLocaleString('en-US', { maximumSignificantDigits: 10 });
}

function deltaSign(value: SerializedNumericValue | null): DeltaSign {
  if (value === null) {
    return 0;
  }
  if (typeof value === 'string') {
    if (/^-0*$/u.test(value) || /^0*$/u.test(value)) {
      return 0;
    }
    return value.startsWith('-') ? -1 : 1;
  }
  return value < 0 ? -1 : value > 0 ? 1 : 0;
}

function colorDelta(value: string, sign: DeltaSign, color: boolean): string {
  if (!color || sign === 0) {
    return value;
  }
  const code = sign < 0 ? 34 : 31;
  return `\u001B[${code}m${value}\u001B[39m`;
}

function formatPercent(value: number | null): string {
  return value === null ? '—' : `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
}

function renderGrid(
  headers: string[],
  values: string[][],
  numericColumns: ReadonlySet<number>,
  deltaColumns: ReadonlySet<number>,
  signs: DeltaSign[],
  color: boolean
): string {
  const widths = headers.map((header, index) =>
    Math.max(header.length, ...values.map(row => row[index]!.length))
  );
  const border = (left: string, middle: string, right: string) =>
    `${left}${widths.map(width => '─'.repeat(width + 2)).join(middle)}${right}`;
  const formatRow = (row: string[], sign: DeltaSign = 0) => {
    const cells = row.map((value, index) => {
      const padded = numericColumns.has(index)
        ? value.padStart(widths[index]!)
        : value.padEnd(widths[index]!);
      return deltaColumns.has(index) ? colorDelta(padded, sign, color) : padded;
    });
    return `│ ${cells.join(' │ ')} │`;
  };
  return [
    border('┌', '┬', '┐'),
    formatRow(headers),
    border('├', '┼', '┤'),
    ...values.map((row, index) => formatRow(row, signs[index])),
    border('└', '┴', '┘'),
  ].join('\n');
}

function metricValues(
  row: Pick<
    QueryDiffRow,
    'metric' | 'quantity' | 'baseline' | 'candidate' | 'delta' | 'deltaPercent'
  >
): string[] {
  return [
    row.metric,
    row.quantity ?? '—',
    formatNumeric(row.baseline, row.metric),
    formatNumeric(row.candidate, row.metric),
    formatNumeric(row.delta, row.metric),
    formatPercent(row.deltaPercent),
  ];
}

function renderOperatorTable(rows: QueryDiffRow[], color: boolean, combined: boolean): string {
  const headers = combined
    ? ['Plan', 'Operator', 'Metric', 'Quantity', 'Baseline', 'Candidate', 'Delta', 'Delta %']
    : ['Metric', 'Quantity', 'Baseline', 'Candidate', 'Delta', 'Delta %'];
  const values = rows.map(row => [
    ...(combined ? [row.scope === 'logical' ? 'Logical' : 'Physical', row.operatorType] : []),
    ...metricValues(row),
  ]);
  const offset = combined ? 2 : 0;
  return renderGrid(
    headers,
    values,
    new Set([2 + offset, 3 + offset, 4 + offset, 5 + offset]),
    new Set([4 + offset, 5 + offset]),
    rows.map(row => deltaSign(row.delta)),
    color
  );
}

function renderSummaryTable(rows: QueryDiffSummary[], color: boolean): string {
  const headers = ['Plan', 'Metric', 'Quantity', 'Baseline', 'Candidate', 'Delta', 'Delta %'];
  const values = rows.map(row => [
    row.scope === 'logical' ? 'Logical' : 'Physical',
    ...metricValues(row),
  ]);
  return renderGrid(
    headers,
    values,
    new Set([3, 4, 5, 6]),
    new Set([5, 6]),
    rows.map(row => deltaSign(row.delta)),
    color
  );
}

function renderGroupedTables(rows: QueryDiffRow[], color: boolean): string {
  const groups = new Map<string, { heading: string; rows: QueryDiffRow[] }>();
  for (const row of rows) {
    const key = `${row.scope}\0${row.operatorType}`;
    const existing = groups.get(key);
    if (existing) {
      existing.rows.push(row);
    } else {
      groups.set(key, {
        heading: `${row.scope === 'logical' ? 'Logical' : 'Physical'} · ${row.operatorType}`,
        rows: [row],
      });
    }
  }
  return [...groups.values()]
    .map(group => `${group.heading}\n${renderOperatorTable(group.rows, color, false)}`)
    .join('\n\n');
}

export function formatQueryDiff(
  result: QueryDiffResult,
  options: QueryDiffFormatOptions = {}
): string {
  const color = options.color ?? false;
  const combinedTable = options.combinedTable ?? true;
  const baselineId = [result.baseline.source, result.baseline.engineId, result.baseline.queryId]
    .filter(Boolean)
    .join(' / ');
  const formatComparison = (comparison: QueryDiffComparison, index: number) => {
    const candidateId = [
      comparison.candidate.source,
      comparison.candidate.engineId,
      comparison.candidate.queryId,
    ]
      .filter(Boolean)
      .join(' / ');
    return [
      `Candidate ${index + 1}: ${candidateId} (${comparison.candidate.durationSeconds}s)`,
      '',
      'Summary · totals across operators by plan type',
      comparison.summary.length > 0
        ? renderSummaryTable(comparison.summary, color)
        : 'No numeric operator metrics found.',
      '',
      'Operator details',
      comparison.rows.length > 0
        ? combinedTable
          ? renderOperatorTable(comparison.rows, color, true)
          : renderGroupedTables(comparison.rows, color)
        : 'No numeric operator metrics found.',
    ];
  };
  const lines = [
    `Baseline: ${baselineId} (${result.baseline.durationSeconds}s)`,
    `Candidates: ${result.comparisons.length}`,
    `Metrics: ${result.metrics.join(', ')}`,
    '',
    ...result.comparisons.flatMap((comparison, index) => [
      ...formatComparison(comparison, index),
      ...(index < result.comparisons.length - 1 ? ['', '═'.repeat(72), ''] : []),
    ]),
    '',
    'Delta = candidate - baseline.',
    '',
    'Limitations:',
    ...result.limitations.map(limitation => `- ${limitation}`),
  ];
  return lines.join('\n');
}
