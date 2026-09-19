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
  threadName: string,
  sourceDomainIds: string[] = [domainId]
): NvtxCatalog['domains'][number] {
  return {
    domain_id: domainId,
    source_domain_ids: sourceDomainIds,
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

const allCatalogLaneRowIds = new Set([nvtxThreadRowId('1', 101), nvtxThreadRowId('3', 303)]);

describe('NVTX resource tree', () => {
  it('keeps the selected domain header above its lanes', () => {
    const tree = buildNvtxTree(catalog, new Set([nvtxThreadRowId('3', 303)]), '3');

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
    const tree = buildNvtxTree(catalog, allCatalogLaneRowIds, null);

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

  it('hides thread lanes with no data anywhere in the full-duration lane row ids', () => {
    const tree = buildNvtxTree(catalog, new Set([nvtxThreadRowId('1', 101)]), null);

    expect(tree?.children?.map(item => item.id)).toEqual([
      nvtxDomainRowId('1'),
      nvtxDomainRowId('3'),
    ]);
    expect(tree?.children?.[0]?.children?.map(item => item.id)).toEqual([
      nvtxThreadRowId('1', 101),
    ]);
    expect(tree?.children?.[1]?.children).toBeUndefined();
  });

  it('appends process and marks lanes after thread rows', () => {
    const viewport = {
      viewport: { start: 0, end: 1 },
      domains: [
        {
          domain_id: '3',
          source_domain_ids: ['3'],
          name: 'CCCL',
          color: '#000000ff',
          lanes: [
            {
              id: 'thread',
              label: 'worker 3',
              identity: { kind: 'thread', source_domain_id: '3', thread_id: 303, depth: 0 },
              ranges: [],
              marks: [],
            },
            {
              id: 'process',
              label: 'Process ranges',
              identity: { kind: 'process', source_domain_id: '3' },
              ranges: [],
              marks: [],
            },
            {
              id: 'marks',
              label: 'Marks',
              identity: { kind: 'marks', source_domain_id: '3' },
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

  it('keeps every raw-source lane under one logical domain row', () => {
    const groupedDomain = nvtxDomain('5', 'CCCL', 42, 'worker', ['5', '172']);
    const groupedCatalog = { domains: [groupedDomain] } satisfies Pick<NvtxCatalog, 'domains'>;
    const lane = (
      id: string,
      identity: NvtxViewportResponse['domains'][number]['lanes'][number]['identity']
    ) => ({ id, label: id, identity, ranges: [], marks: [] });
    const viewport = {
      viewport: { start: 0, end: 1 },
      domains: [
        {
          domain_id: '5',
          source_domain_ids: ['5', '172'],
          name: 'CCCL',
          color: '#000000ff',
          lanes: [
            lane('thread-172', {
              kind: 'thread',
              source_domain_id: '172',
              thread_id: 42,
              depth: 0,
            }),
            lane('thread-5-depth-1', {
              kind: 'thread',
              source_domain_id: '5',
              thread_id: 42,
              depth: 1,
            }),
            lane('thread-5-depth-0', {
              kind: 'thread',
              source_domain_id: '5',
              thread_id: 42,
              depth: 0,
            }),
            lane('process-5', { kind: 'process', source_domain_id: '5' }),
            lane('process-172', { kind: 'process', source_domain_id: '172' }),
            lane('marks-5', { kind: 'marks', source_domain_id: '5' }),
            lane('marks-172', { kind: 'marks', source_domain_id: '172' }),
          ],
        },
      ],
      statistics: [],
    } satisfies NvtxViewportResponse;

    const lanesByRowId = indexNvtxLanes(viewport);
    const tree = buildNvtxTree(groupedCatalog, new Set(lanesByRowId.keys()), null);

    expect(tree?.children).toHaveLength(1);
    expect(tree?.children?.[0]?.id).toBe(nvtxDomainRowId('5'));
    expect(lanesByRowId.get(nvtxThreadRowId('5', 42))?.map(item => item.id)).toEqual([
      'thread-5-depth-0',
      'thread-5-depth-1',
      'thread-172',
    ]);
    expect(lanesByRowId.get(nvtxProcessRowId('5'))?.map(item => item.id)).toEqual([
      'process-5',
      'process-172',
    ]);
    expect(lanesByRowId.get(nvtxMarksRowId('5'))?.map(item => item.id)).toEqual([
      'marks-5',
      'marks-172',
    ]);
  });

  it('filters labels while retaining the path to direct matches', () => {
    const tree = buildNvtxTree(catalog, allCatalogLaneRowIds, null)!;
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
    const tree = buildNvtxTree(catalog, allCatalogLaneRowIds, null)!;
    const result = filterNvtxTree(tree, ' \t ');

    expect(result.filteredTree).toBe(tree);
    expect(result.isActive).toBe(false);
    expect(result.directMatchIds).toEqual(new Set());
    expect(result.matchCount).toBe(0);
  });

  it('supports comma-separated OR groups and space-separated AND terms', () => {
    const tree = buildNvtxTree(catalog, allCatalogLaneRowIds, null)!;
    const result = filterNvtxTree(tree, 'libcudf missing, CCCL worker');

    expect(result.directMatchIds).toEqual(new Set([nvtxThreadRowId('3', 303)]));
    expect(result.matchCount).toBe(1);
  });
});
