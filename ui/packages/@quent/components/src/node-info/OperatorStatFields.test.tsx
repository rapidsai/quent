// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { parseCustomStatistics } from '../lib/queryBundle.utils';
import { OperatorStatFields } from './OperatorStatFields';

function show(attributes: unknown[]) {
  render(
    <OperatorStatFields
      operator={{
        nodeId: 'actor-1',
        label: 'Join',
        operationType: 'join',
        statistics: parseCustomStatistics({
          statistics: { custom_statistics: attributes.map(value => ({ value, quantity: null })) },
        }),
      }}
    />
  );
}

describe('OperatorStatFields', () => {
  it('uses producer names as nested headings and retains field order and repeated names', () => {
    show([
      {
        key: 'Join decomposition',
        value: {
          Struct: [
            {
              key: 'Probe',
              value: {
                Struct: [
                  { key: 'last', value: { U64: 2 } },
                  { key: 'first', value: { U64: 1 } },
                ],
              },
            },
            { key: 'Build', value: { Struct: [{ key: 'status', value: { String: 'Ready' } }] } },
            {
              key: 'Build',
              value: { Struct: [{ key: 'status', value: { String: 'Completed' } }] },
            },
          ],
        },
      },
      { key: 'execution_status', value: { String: 'Complete' } },
    ]);
    expect(screen.getAllByRole('heading').map(h => h.textContent)).toEqual([
      'Join decomposition',
      'Probe',
      'Build',
      'Build',
    ]);
    const probe = screen.getByRole('region', { name: 'Probe' });
    expect(
      within(probe)
        .getAllByRole('term')
        .map(t => t.textContent)
    ).toEqual(['last:', 'first:']);
    expect(screen.getByText('Ready')).toBeVisible();
    expect(screen.getByText('Completed')).toBeVisible();
    expect(screen.getByText('execution status:')).toBeVisible();
    expect(screen.queryByText('[object Object]')).not.toBeInTheDocument();
  });

  it('distinguishes empty structs, empty lists, null, and zero', () => {
    show([
      { key: 'Empty group', value: { Struct: [] } },
      { key: 'empty_list', value: { List: { U64: [] } } },
      { key: 'missing', value: null },
      { key: 'count', value: { U64: 0 } },
    ]);
    expect(screen.getByRole('heading', { name: 'Empty group' })).toBeVisible();
    expect(screen.getByText('No fields')).toBeVisible();
    expect(screen.getByText('Empty list')).toBeVisible();
    expect(screen.getByText('—')).toBeVisible();
    expect(screen.getByText('0')).toBeVisible();
  });

  it('renders scalar lists and lists of structs without stringifying them', () => {
    show([
      { key: 'values', value: { List: { U64: [0, 2] } } },
      {
        key: 'inputs',
        value: {
          List: {
            Struct: [[{ key: 'rows', value: { U64: 3 } }], [{ key: 'rows', value: { U64: 4 } }]],
          },
        },
      },
    ]);
    expect(
      within(screen.getByRole('list', { name: 'values values' }))
        .getAllByRole('listitem')
        .map(li => li.textContent)
    ).toEqual(['0', '2']);
    expect(
      within(screen.getByRole('list', { name: 'inputs values' }))
        .getAllByRole('definition')
        .map(dd => dd.textContent)
    ).toEqual(['3', '4']);
  });
});

it('shows producer-defined decomposition from declaration attributes before terminal statistics', () => {
  render(
    <OperatorStatFields
      operator={{
        nodeId: 'one',
        label: 'Fused',
        operationType: 'fused',
        attributes: [
          {
            key: 'Fused decomposition',
            value: {
              kind: 'struct',
              fields: [
                {
                  key: 'Projection',
                  value: {
                    kind: 'struct',
                    fields: [{ key: 'expression', value: 'price * discount' }],
                  },
                },
              ],
            },
          },
        ],
        statistics: [
          { key: 'Execution', value: { kind: 'struct', fields: [{ key: 'tasks', value: 2 }] } },
        ],
      }}
    />
  );
  expect(screen.getAllByRole('heading').map(h => h.textContent)).toEqual([
    'Fused decomposition',
    'Projection',
    'Execution',
  ]);
  expect(screen.getByText('price * discount')).toBeVisible();
});
