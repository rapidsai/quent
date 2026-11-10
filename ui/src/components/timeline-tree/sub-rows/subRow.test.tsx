// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { TreeTableItem } from '@quent/components';
import { EntityTypeKey } from '@quent/utils';
import { render, screen } from '@testing-library/react';
import {
  createSyntheticSubRow,
  createTimelineSubRow,
  createTimelineSubRowDisclosure,
  mapTreeItems,
  TimelineSubRowLabel,
} from './subRow';

function item(id: string, type: string, children?: TreeTableItem[]): TreeTableItem {
  return { id, type, entity: {} as TreeTableItem['entity'], children };
}

describe('mapTreeItems', () => {
  it('appends a sibling after matching children', () => {
    const root = item('root', 'group', [item('res-1', EntityTypeKey.Resource)]);
    const next = mapTreeItems(root, (_self, children) => {
      const rewritten: TreeTableItem[] = [];
      for (const child of children) {
        rewritten.push(child);
        if (child.type === EntityTypeKey.Resource) {
          rewritten.push(createSyntheticSubRow(`${child.id}-sub`, 'sub'));
        }
      }
      return rewritten;
    });
    expect(next.children?.map(child => child.id)).toEqual(['res-1', 'res-1-sub']);
  });

  it('prepends a child on matching nodes, including leaves', () => {
    const root = item('root', 'group', [item('worker-1', EntityTypeKey.Resource)]);
    const next = mapTreeItems(root, (self, children) =>
      self.id === 'worker-1'
        ? [createSyntheticSubRow(`${self.id}-ops`, 'ops'), ...children]
        : children
    );
    expect(next.children?.[0]?.children?.map(child => child.id)).toEqual(['worker-1-ops']);
  });
});

describe('createTimelineSubRow', () => {
  it('keeps its label as plain text', () => {
    const subRow = createTimelineSubRow({
      id: 'demo',
      rowType: 'demo-row',
      label: 'Demo',
      injectRows: root => root,
      renderTimeline: () => null,
    });

    expect(subRow.label).toBe('Demo');
  });

  it('labels the disclosure row but leaves the chart row beneath it unlabeled', () => {
    const subRow = createTimelineSubRow({
      id: 'demo',
      rowType: 'demo-row',
      label: 'Demo',
      injectRows: root => root,
      renderTimeline: () => null,
    });
    const disclosure = item('__demo_row_disclosure__owner', 'demo-row-disclosure') as never;
    const chartRow = item('__demo_row__owner', 'demo-row') as never;

    expect(subRow.renderLabel(disclosure)).not.toBeNull();
    expect(subRow.renderLabel(chartRow)).toBeNull();
  });
});

describe('createTimelineSubRowDisclosure', () => {
  it('leaves the chart row to be drawn by its disclosure row', () => {
    const chartRow = createSyntheticSubRow('__demo_row__owner', 'demo-row');
    const disclosure = createTimelineSubRowDisclosure('owner', 'demo-row', chartRow);

    expect(disclosure.children?.[0]?.id).toBe('__demo_row__owner');
    expect(disclosure.children?.[0]?.rendersInParent).toBe(true);
  });
});

describe('createTimelineSubRow timelines', () => {
  const subRow = createTimelineSubRow({
    id: 'demo',
    rowType: 'demo-row',
    label: 'Demo',
    injectRows: root => root,
    renderTimeline: chartRow => `chart for ${chartRow.id}`,
  });
  const disclosure = createTimelineSubRowDisclosure(
    'owner',
    'demo-row',
    createSyntheticSubRow('__demo_row__owner', 'demo-row')
  ) as never;

  it('shows the chart in the disclosure row while it is open', () => {
    expect(subRow.renderTimeline(disclosure, true)).toBe('chart for __demo_row__owner');
  });

  it('shows nothing in the disclosure row while it is closed', () => {
    expect(subRow.renderTimeline(disclosure, false)).toBeNull();
    expect(subRow.renderTimeline(disclosure)).toBeNull();
  });
});

describe('TimelineSubRowLabel', () => {
  it('shows only the label by default', () => {
    render(<TimelineSubRowLabel>Entities</TimelineSubRowLabel>);

    expect(screen.getByText('Entities')).toBeInTheDocument();
    expect(screen.queryByText('Worker: worker-0')).not.toBeInTheDocument();
  });

  it('shows the description in the same column as the label', () => {
    render(<TimelineSubRowLabel description="Worker: worker-0">Entities</TimelineSubRowLabel>);

    expect(screen.getByText('Worker: worker-0').parentElement).toBe(
      screen.getByText('Entities').parentElement
    );
  });
});
