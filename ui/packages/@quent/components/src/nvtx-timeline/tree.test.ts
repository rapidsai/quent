// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { NvtxCatalog, NvtxViewportResponse } from '@quent/utils';
import {
  buildNvtxTree,
  filterNvtxTree,
  indexNvtxLanes,
  NVTX_DOMAIN_ROW_TYPE,
  NVTX_LANE_ROW_TYPE,
  nvtxDomainRowId,
  nvtxDomainMeta,
  nvtxLaneLabel,
  nvtxMarksRowId,
  nvtxProcessRowId,
  nvtxThreadRowId,
} from './utils';

function nvtxDomain(
  domainId: string,
  name: string,
  threadId: number,
  threadName: string
): NvtxCatalog['domains'][number] {
  return {
    domain_id: domainId,
    name,
    color: '#000000ff',
    threads: [{ thread_id: threadId, name: threadName }],
    categories: [],
    has_uncategorized: true,
  };
}

const catalog = {
  domains: [nvtxDomain('1', 'libcudf', 101, 'worker 1'), nvtxDomain('3', 'CCCL', 303, 'worker 3')],
} satisfies Pick<NvtxCatalog, 'domains'>;

describe('NVTX resource tree', () => {
  it('keeps the selected domain header above its lanes', () => {
    const tree = buildNvtxTree(catalog, new Set(), '3');

    expect(tree?.children).toEqual([
      expect.objectContaining({
        id: nvtxDomainRowId('3'),
        type: NVTX_DOMAIN_ROW_TYPE,
        entity: expect.objectContaining({
          nvtxKind: 'domain',
          domain: catalog.domains[1],
        }),
        children: [
          expect.objectContaining({
            id: nvtxThreadRowId('3', 303),
            type: NVTX_LANE_ROW_TYPE,
            entity: expect.objectContaining({
              nvtxKind: 'thread',
              domain: catalog.domains[1],
              thread: catalog.domains[1]?.threads[0],
            }),
          }),
        ],
      }),
    ]);
    expect(nvtxDomainMeta(tree!.children![0]!.entity)).toEqual({ name: 'CCCL', color: '#000000' });
    expect(nvtxLaneLabel(tree!.children![0]!.children![0]!.entity)).toBe('worker 3');
  });

  it('keeps each domain in a sub-tree when showing all domains', () => {
    const tree = buildNvtxTree(catalog, new Set(), null);

    expect(tree?.children).toEqual([
      expect.objectContaining({
        id: nvtxDomainRowId('1'),
        type: NVTX_DOMAIN_ROW_TYPE,
        children: [expect.objectContaining({ id: nvtxThreadRowId('1', 101) })],
      }),
      expect.objectContaining({
        id: nvtxDomainRowId('3'),
        type: NVTX_DOMAIN_ROW_TYPE,
        children: [expect.objectContaining({ id: nvtxThreadRowId('3', 303) })],
      }),
    ]);
    const domainRow = tree!.children![1]!;
    expect(nvtxDomainMeta(domainRow.entity)).toEqual({ name: 'CCCL', color: '#000000' });
    expect(nvtxLaneLabel(domainRow.children![0]!.entity)).toBe('worker 3');
  });

  it('appends process and marks lanes after thread rows', () => {
    const viewport = {
      viewport: { start: 0, end: 1 },
      domains: [
        {
          domain_id: '3',
          name: 'CCCL',
          color: '#000000ff',
          lanes: [
            {
              id: 'process',
              label: 'Process ranges',
              identity: { kind: 'process' },
              ranges: [],
              marks: [],
            },
            {
              id: 'marks',
              label: 'Marks',
              identity: { kind: 'marks' },
              ranges: [],
              marks: [],
            },
          ],
        },
      ],
      statistics: [],
    } satisfies NvtxViewportResponse;
    const lanesByRowId = indexNvtxLanes(viewport);
    const tree = buildNvtxTree(catalog, new Set(lanesByRowId.keys()), '3');

    expect(tree?.children?.map(item => item.id)).toEqual([nvtxDomainRowId('3')]);
    expect(tree?.children?.[0]?.children?.map(item => item.id)).toEqual([
      nvtxThreadRowId('3', 303),
      nvtxProcessRowId('3'),
      nvtxMarksRowId('3'),
    ]);
    expect(tree?.children?.[0]?.children?.map(item => nvtxLaneLabel(item.entity))).toEqual([
      'worker 3',
      'Process ranges',
      'Marks',
    ]);
  });

  it('filters labels while retaining the path to direct matches', () => {
    const tree = buildNvtxTree(catalog, new Set(), null)!;
    const result = filterNvtxTree(tree, 'worker 3');

    expect(result.matchCount).toBe(1);
    expect(result.directMatchIds).toEqual(new Set([nvtxThreadRowId('3', 303)]));
    expect(result.filteredTree?.children).toEqual([
      expect.objectContaining({
        id: nvtxDomainRowId('3'),
        children: [expect.objectContaining({ id: nvtxThreadRowId('3', 303) })],
      }),
    ]);
  });

  it('preserves the original tree for whitespace-only searches', () => {
    const tree = buildNvtxTree(catalog, new Set(), null)!;
    const result = filterNvtxTree(tree, ' \t ');

    expect(result.filteredTree).toBe(tree);
    expect(result.isActive).toBe(false);
    expect(result.directMatchIds).toEqual(new Set());
    expect(result.matchCount).toBe(0);
  });

  it('supports comma-separated OR groups and space-separated AND terms', () => {
    const tree = buildNvtxTree(catalog, new Set(), null)!;
    const result = filterNvtxTree(tree, 'libcudf missing, CCCL worker');

    expect(result.directMatchIds).toEqual(new Set([nvtxThreadRowId('3', 303)]));
    expect(result.matchCount).toBe(1);
  });
});
