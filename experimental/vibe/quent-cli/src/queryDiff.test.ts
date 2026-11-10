// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import { getApiClient, setApiClient, type ApiClient } from '@quent/client';
import type { EntityRef, QueryBundle } from '@quent/utils';
import type { DiscoveryApi, SelectFromList, SelectFromQueryTree } from './askSelection';
import {
  createHttpDiscoveryApi,
  queryDiffCommand,
  resolveDatabaseRunIds,
  resolveQueryDiffMetrics,
  resolveQueryDiffSelections,
  resolveSourcedQueryDiffSelections,
  runQueryDiffCommand,
  type QueryDiffSource,
} from './queryDiff';

const originalClient = getApiClient();

afterEach(() => {
  setApiClient(originalClient);
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function discoveryApi(): DiscoveryApi {
  return {
    fetchListEngines: vi.fn().mockResolvedValue({
      items: [
        { id: 'engine-1', instance_name: 'Baseline' },
        { id: 'engine-2', instance_name: 'Candidate' },
      ],
      initial_group_by_attribute: null,
    }),
    fetchListQueries: vi.fn(async engineId => ({
      items: [
        {
          id: engineId === 'engine-1' ? 'query-1' : 'query-2',
          instance_name: engineId === 'engine-1' ? 'Old query' : 'New query',
          custom_attributes: [],
        },
      ],
      initial_group_by_attribute: null,
    })),
    fetchQueryBundle: vi.fn(),
  } as unknown as DiscoveryApi;
}

function metricBundle(queryId: string, metrics: string[]): QueryBundle<EntityRef> {
  return {
    query_id: queryId,
    duration_s: 1,
    entities: {
      plans: { logical: { id: 'logical', worker_id: null } },
      operators: {
        operator: {
          id: 'operator',
          plan_id: 'logical',
          parent_operator_ids: [],
          operator_type_name: 'Scan',
          active_span: null,
          statistics: {
            custom_statistics: metrics.map(metric => ({
              value: { key: metric, value: { U64: 1 } },
              quantity: null,
            })),
          },
        },
      },
    },
  } as unknown as QueryBundle<EntityRef>;
}

describe('query diff selections', () => {
  it('keeps query and metric selection interactive in JSON mode', async () => {
    const api = discoveryApi();
    api.fetchQueryBundle = vi.fn(async (_engineId, queryId) =>
      metricBundle(queryId, ['output_rows'])
    );
    setApiClient(api as unknown as ApiClient);
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValueOnce(['engine-1\0query-1'])
      .mockResolvedValueOnce(['engine-2\0query-2']);
    const selectMetrics = vi.fn().mockResolvedValue(['all']);
    const write = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);

    await runQueryDiffCommand(['--json'], {
      select: vi.fn<SelectFromList>(),
      selectTree,
      selectMetrics,
      inputText: vi.fn(),
    });

    expect(selectTree).toHaveBeenCalledTimes(2);
    expect(selectMetrics).toHaveBeenCalledOnce();
    const output = JSON.parse(String(write.mock.calls[0]![0]));
    expect(output.schemaVersion).toBe(2);
    expect(output.command).toBe('query-diff');
  });

  it('opens the non-JSON prompt when another engine query list fails', async () => {
    const api = discoveryApi();
    vi.mocked(api.fetchListQueries).mockImplementation(async engineId => {
      if (engineId === 'engine-2') {
        throw new Error('API Error: 500 Internal Server Error');
      }
      return {
        items: [
          {
            id: 'query-1',
            instance_name: 'Old query',
            custom_attributes: [],
            start_unix_ns: null,
            planning_s: null,
            executing_s: null,
            completed_s: null,
          },
          {
            id: 'query-2',
            instance_name: 'New query',
            custom_attributes: [],
            start_unix_ns: null,
            planning_s: null,
            executing_s: null,
            completed_s: null,
          },
        ],
        initial_group_by_attribute: null,
      };
    });
    api.fetchQueryBundle = vi.fn(async (_engineId, queryId) =>
      metricBundle(queryId, ['output_rows'])
    );
    setApiClient(api as unknown as ApiClient);
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValueOnce(['engine-1\0query-1'])
      .mockResolvedValueOnce(['engine-1\0query-2']);
    const selectMetrics = vi.fn().mockResolvedValue(['all']);
    vi.spyOn(process.stderr, 'write').mockImplementation(() => true);
    const write = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);

    await runQueryDiffCommand([], {
      select: vi.fn<SelectFromList>(),
      selectTree,
      selectMetrics,
      inputText: vi.fn(),
    });

    expect(selectTree).toHaveBeenCalledTimes(2);
    expect(selectMetrics).toHaveBeenCalledOnce();
    expect(write).toHaveBeenCalledOnce();
  });

  it('selects the baseline and multiple candidates from the all-engine query tree', async () => {
    const api = discoveryApi();
    const select = vi.fn<SelectFromList>();
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValueOnce(['engine-1\0query-1'])
      .mockResolvedValueOnce(['engine-2\0query-2']);

    await expect(resolveQueryDiffSelections({}, api, select, selectTree)).resolves.toEqual({
      baseline: { engineId: 'engine-1', queryId: 'query-1' },
      candidates: [{ engineId: 'engine-2', queryId: 'query-2' }],
    });
    expect(select).not.toHaveBeenCalled();
    expect(selectTree).toHaveBeenNthCalledWith(
      1,
      'Select the baseline query',
      expect.any(Array),
      false
    );
    expect(selectTree).toHaveBeenNthCalledWith(
      2,
      'Select candidate queries',
      expect.any(Array),
      true
    );
  });

  it('accepts any number of candidate query IDs on one engine', async () => {
    const api = discoveryApi();
    const select = vi.fn<SelectFromList>();
    const selectTree = vi.fn<SelectFromQueryTree>();

    await expect(
      resolveQueryDiffSelections(
        {
          engine: 'engine-1',
          'baseline-query': 'query-1',
          'candidate-query': ['query-2', 'query-3', 'query-4'],
        },
        api,
        select,
        selectTree
      )
    ).resolves.toEqual({
      baseline: { engineId: 'engine-1', queryId: 'query-1' },
      candidates: [
        { engineId: 'engine-1', queryId: 'query-2' },
        { engineId: 'engine-1', queryId: 'query-3' },
        { engineId: 'engine-1', queryId: 'query-4' },
      ],
    });
    expect(api.fetchListEngines).not.toHaveBeenCalled();
    expect(select).not.toHaveBeenCalled();
    expect(selectTree).not.toHaveBeenCalled();
  });

  it('accepts repeatable qualified candidates across engines', async () => {
    const api = discoveryApi();

    await expect(
      resolveQueryDiffSelections(
        {
          'baseline-engine': 'engine-1',
          'baseline-query': 'query-1',
          candidate: ['engine-2:query-2', 'engine-3:query-3'],
        },
        api,
        vi.fn<SelectFromList>(),
        vi.fn<SelectFromQueryTree>()
      )
    ).resolves.toEqual({
      baseline: { engineId: 'engine-1', queryId: 'query-1' },
      candidates: [
        { engineId: 'engine-2', queryId: 'query-2' },
        { engineId: 'engine-3', queryId: 'query-3' },
      ],
    });
  });

  it('fetches and emits every candidate through the JSON command', async () => {
    const fetch = vi.fn(async (_engineId: string, queryId: string) => {
      return {
        query_id: queryId,
        duration_s: 1,
        entities: {
          plans: {
            logical: { id: 'logical', worker_id: null },
          },
          operators: {
            operator: {
              id: 'operator',
              plan_id: 'logical',
              parent_operator_ids: [],
              operator_type_name: 'Scan',
              active_span: { start: 0, end: 1 },
              statistics: { custom_statistics: [] },
            },
          },
        },
      } as unknown as QueryBundle<EntityRef>;
    });
    setApiClient({ fetchQueryBundle: fetch } as unknown as ApiClient);
    const write = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);

    await queryDiffCommand.run([
      '--engine',
      'engine-1',
      '--baseline-query',
      'query-1',
      '--candidate-query',
      'query-2',
      '--candidate-query',
      'query-3',
      '--metric',
      'all',
      '--json',
    ]);

    expect(fetch.mock.calls).toEqual([
      ['engine-1', 'query-1'],
      ['engine-1', 'query-2'],
      ['engine-1', 'query-3'],
    ]);
    const output = JSON.parse(String(write.mock.calls[0]![0]));
    expect(output.schemaVersion).toBe(2);
    expect(output.data.comparisons).toHaveLength(2);
    expect(output.data.metrics).toEqual(['active_span_s']);
  });

  it('uses a combined operator table by default and allows grouped output', async () => {
    setApiClient({
      fetchQueryBundle: vi.fn(async (_engineId: string, queryId: string) =>
        metricBundle(queryId, ['output_rows'])
      ),
    } as unknown as ApiClient);
    const write = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);
    const args = [
      '--engine',
      'engine-1',
      '--baseline-query',
      'query-1',
      '--candidate-query',
      'query-2',
      '--metric',
      'output_rows',
    ];

    await queryDiffCommand.run(args);
    expect(String(write.mock.calls[0]![0])).toContain('│ Plan');
    expect(String(write.mock.calls[0]![0])).toContain('│ Operator');

    write.mockClear();
    await queryDiffCommand.run([...args, '--no-combined-table']);
    expect(String(write.mock.calls[0]![0])).toContain('Logical · Scan\n┌');
  });

  it('requires candidate engine counts to be unambiguous', async () => {
    await expect(
      resolveQueryDiffSelections(
        {
          'baseline-engine': 'engine-1',
          'baseline-query': 'query-1',
          'candidate-engine': ['engine-2', 'engine-3'],
          'candidate-query': ['query-2', 'query-3', 'query-4'],
        },
        discoveryApi(),
        vi.fn<SelectFromList>(),
        vi.fn<SelectFromQueryTree>()
      )
    ).rejects.toThrow(
      'Pass one --candidate-engine for all candidate queries or one --candidate-engine per query.'
    );
  });

  it('offers only metrics shared by every selected query', async () => {
    const selectMetrics = vi.fn().mockResolvedValue(['output_rows']);
    const metrics = await resolveQueryDiffMetrics(
      {},
      [
        metricBundle('baseline', ['bytes_read', 'output_rows']),
        metricBundle('candidate-1', ['output_rows', 'selectivity']),
        metricBundle('candidate-2', ['output_rows', 'spill_bytes']),
      ],
      selectMetrics
    );

    expect(metrics).toEqual(['output_rows']);
    expect(selectMetrics).toHaveBeenCalledWith(
      'Select metrics to compare',
      [
        { value: '\0all-metrics', label: 'All metrics (1)' },
        { value: 'output_rows', label: 'output_rows' },
      ],
      '\0all-metrics'
    );
  });

  it('supports all and rejects metrics absent from any query', async () => {
    const bundles = [
      metricBundle('baseline', ['bytes_read', 'output_rows']),
      metricBundle('candidate', ['bytes_read', 'output_rows']),
    ];

    await expect(resolveQueryDiffMetrics({ metric: ['all'] }, bundles)).resolves.toEqual([
      'bytes_read',
      'output_rows',
    ]);
    await expect(resolveQueryDiffMetrics({ metric: ['selectivity'] }, bundles)).rejects.toThrow(
      'Metrics not available in every selected query: selectivity'
    );
  });

  it('keeps colliding engine and query IDs distinct across sources', async () => {
    const api = discoveryApi();
    const sources: QueryDiffSource[] = [
      { id: 'local', label: 'local', api },
      { id: '6647', label: 'db 6647', api },
    ];
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValueOnce(['local\0engine-1\0query-1'])
      .mockResolvedValueOnce(['6647\0engine-1\0query-1']);

    await expect(
      resolveSourcedQueryDiffSelections({}, sources, vi.fn<SelectFromList>(), selectTree)
    ).resolves.toEqual({
      baseline: {
        sourceId: 'local',
        sourceLabel: 'local',
        engineId: 'engine-1',
        queryId: 'query-1',
      },
      candidates: [
        {
          sourceId: '6647',
          sourceLabel: 'db 6647',
          engineId: 'engine-1',
          queryId: 'query-1',
        },
      ],
    });
    const candidateChoices = selectTree.mock.calls[1]![1];
    expect(candidateChoices).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          sourceId: '6647',
          engineLabel: 'db 6647 · Baseline (engine-1)',
        }),
      ])
    );
  });

  it('keeps healthy sources available and reports failed source discovery', async () => {
    const api = discoveryApi();
    const unavailableApi = {
      ...discoveryApi(),
      fetchListEngines: vi.fn().mockRejectedValue(new Error('source unavailable')),
    };
    const write = vi.spyOn(process.stderr, 'write').mockImplementation(() => true);
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValueOnce(['local\0engine-1\0query-1'])
      .mockResolvedValueOnce(['local\0engine-2\0query-2']);

    await expect(
      resolveSourcedQueryDiffSelections(
        {},
        [
          { id: 'local', label: 'local', api },
          { id: 'broken', label: 'db broken', api: unavailableApi },
        ],
        vi.fn<SelectFromList>(),
        selectTree
      )
    ).resolves.toMatchObject({
      baseline: { sourceId: 'local', engineId: 'engine-1', queryId: 'query-1' },
      candidates: [{ sourceId: 'local', engineId: 'engine-2', queryId: 'query-2' }],
    });
    expect(write).toHaveBeenCalledWith(
      'Warning: unable to discover queries from source "db broken": source unavailable\n'
    );
  });

  it('surfaces discovery failures for an explicitly selected source', async () => {
    const api = discoveryApi();
    const unavailableApi = {
      ...discoveryApi(),
      fetchListEngines: vi.fn().mockRejectedValue(new Error('selected source unavailable')),
    };
    vi.spyOn(process.stderr, 'write').mockImplementation(() => true);

    await expect(
      resolveSourcedQueryDiffSelections(
        { 'baseline-source': 'broken' },
        [
          { id: 'local', label: 'local', api },
          { id: 'broken', label: 'db broken', api: unavailableApi },
        ],
        vi.fn<SelectFromList>(),
        vi.fn<SelectFromQueryTree>()
      )
    ).rejects.toThrow('selected source unavailable');
  });

  it('resolves explicit DB-to-DB sources', async () => {
    const api = discoveryApi();
    const sources: QueryDiffSource[] = [
      { id: '6647', label: 'db 6647', api },
      { id: '6650', label: 'db 6650', api },
    ];

    await expect(
      resolveSourcedQueryDiffSelections(
        {
          'baseline-source': '6647',
          'baseline-engine': 'engine-1',
          'baseline-query': 'query-1',
          'candidate-source': ['6650'],
          'candidate-engine': ['engine-2'],
          'candidate-query': ['query-2'],
        },
        sources,
        vi.fn<SelectFromList>(),
        vi.fn<SelectFromQueryTree>()
      )
    ).resolves.toEqual({
      baseline: {
        sourceId: '6647',
        sourceLabel: 'db 6647',
        engineId: 'engine-1',
        queryId: 'query-1',
      },
      candidates: [
        {
          sourceId: '6650',
          sourceLabel: 'db 6650',
          engineId: 'engine-2',
          queryId: 'query-2',
        },
      ],
    });
  });

  it('infers the only DB source for explicit selections', async () => {
    const api = discoveryApi();

    await expect(
      resolveSourcedQueryDiffSelections(
        {
          engine: 'engine-1',
          'baseline-query': 'query-1',
          'candidate-query': ['query-2'],
        },
        [{ id: '6647', label: 'db 6647', api }],
        vi.fn<SelectFromList>(),
        vi.fn<SelectFromQueryTree>()
      )
    ).resolves.toMatchObject({
      baseline: { sourceId: '6647' },
      candidates: [{ sourceId: '6647' }],
    });
  });

  it('preserves the local-only query tree labels', async () => {
    const api = discoveryApi();
    const selectTree = vi
      .fn<SelectFromQueryTree>()
      .mockResolvedValueOnce(['engine-1\0query-1'])
      .mockResolvedValueOnce(['engine-2\0query-2']);

    await resolveSourcedQueryDiffSelections(
      {},
      [{ id: 'local', label: 'local', api }],
      vi.fn<SelectFromList>(),
      selectTree
    );

    expect(selectTree.mock.calls[0]![1][0]!.engineLabel).toBe('Baseline (engine-1)');
    expect(selectTree.mock.calls[0]![1][0]!.sourceId).toBeUndefined();
  });
});

describe('database run selection', () => {
  it('deduplicates explicit runs without prompting', async () => {
    const input = vi.fn();

    await expect(
      resolveDatabaseRunIds({ 'db-run': ['6647', '6650', '6647'] }, input)
    ).resolves.toEqual(['6647', '6650']);
    expect(input).not.toHaveBeenCalled();
  });

  it('accepts comma-separated interactive run IDs', async () => {
    const input = vi.fn().mockResolvedValue('6647, 6650, 6647');

    await expect(resolveDatabaseRunIds({ db: true }, input)).resolves.toEqual(['6647', '6650']);
  });

  it('accepts interactive run IDs in JSON mode', async () => {
    const input = vi.fn().mockResolvedValue('6647, 6650, 6647');

    await expect(resolveDatabaseRunIds({ db: true, json: true }, input)).resolves.toEqual([
      '6647',
      '6650',
    ]);
    expect(input).toHaveBeenCalledOnce();
  });
});

describe('quent-open discovery compatibility', () => {
  it('normalizes current catalog responses', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async input => {
        const url = String(input);
        if (url.endsWith('/engines?with_metadata=true')) {
          return new Response(
            JSON.stringify({
              items: [{ id: 'engine-1', instance_name: 'Engine' }],
              initial_group_by_attribute: 'frontend',
            })
          );
        }
        if (url.endsWith('/engines/engine-1/queries')) {
          return new Response(
            JSON.stringify({
              items: [{ id: 'query-1', instance_name: 'Query' }],
              initial_group_by_attribute: 'workload',
            })
          );
        }
        throw new Error(`Unexpected request: ${url}`);
      })
    );
    const api = createHttpDiscoveryApi('http://viewer.test/api/');

    await expect(api.fetchListEngines()).resolves.toMatchObject({
      items: [{ id: 'engine-1' }],
      initial_group_by_attribute: 'frontend',
    });
    await expect(api.fetchListQueries('engine-1')).resolves.toMatchObject({
      items: [{ id: 'query-1' }],
      initial_group_by_attribute: 'workload',
    });
  });

  it('discovers queries through legacy query groups', async () => {
    const fetchMock = vi.fn(async input => {
      const url = String(input);
      if (url.endsWith('/engines?with_metadata=true')) {
        return new Response(JSON.stringify([{ id: 'engine-1', instance_name: 'Engine' }]));
      }
      if (url.endsWith('/engines/engine-1/queries')) {
        return new Response('<!doctype html><html><body>Viewer</body></html>', {
          status: 200,
          headers: { 'Content-Type': 'text/html' },
        });
      }
      if (url.endsWith('/engines/engine-1/query-groups')) {
        return new Response(JSON.stringify([{ id: 'group-1' }, { id: 'group-2' }]));
      }
      if (url.endsWith('/query_group/group-1/queries')) {
        return new Response(JSON.stringify([{ id: 'query-1', instance_name: 'First' }]));
      }
      if (url.endsWith('/query_group/group-2/queries')) {
        return new Response(JSON.stringify([{ id: 'query-2', instance_name: 'Second' }]));
      }
      throw new Error(`Unexpected request: ${url}`);
    });
    vi.stubGlobal('fetch', fetchMock);
    const api = createHttpDiscoveryApi('http://viewer.test/api');

    await expect(api.fetchListEngines()).resolves.toEqual({
      items: [{ id: 'engine-1', instance_name: 'Engine' }],
      initial_group_by_attribute: null,
    });
    await expect(api.fetchListQueries('engine-1')).resolves.toEqual({
      items: [
        { id: 'query-1', instance_name: 'First' },
        { id: 'query-2', instance_name: 'Second' },
      ],
      initial_group_by_attribute: null,
    });
  });
});
