// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it, vi } from 'vitest';
import type { SelectFromList } from './askSelection';
import type { QuentOpenDbHandle } from './quentOpenDb';
import {
  databaseQuerySources,
  selectQuerySource,
  withQuerySources,
  type QuerySource,
  type QuerySourceApi,
  type QuerySourceRuntime,
} from './querySources';

const api = {} as QuerySourceApi;

function sources(): QuerySource[] {
  return [
    { id: 'local', label: 'local', api },
    { id: '6647', label: 'db 6647', api },
  ];
}

describe('query sources', () => {
  it('resolves an explicit source without prompting', async () => {
    const select = vi.fn<SelectFromList>();

    await expect(selectQuerySource(sources(), '6647', select)).resolves.toMatchObject({
      id: '6647',
    });
    expect(select).not.toHaveBeenCalled();
  });

  it('prompts with every available source', async () => {
    const select = vi.fn<SelectFromList>().mockResolvedValue('6647');

    await expect(selectQuerySource(sources(), undefined, select)).resolves.toMatchObject({
      id: '6647',
    });
    expect(select).toHaveBeenCalledWith('Select a query source', [
      { value: 'local', label: 'local' },
      { value: '6647', label: 'db 6647' },
    ]);
  });

  it('registers every viewer produced by database runs', () => {
    const handles: QuentOpenDbHandle[] = [
      {
        run: '6647',
        viewers: [
          { label: 'first viewer', apiBaseUrl: 'http://first.test/api' },
          { label: 'second viewer', apiBaseUrl: 'http://second.test/api' },
        ],
        stop: vi.fn(),
      },
      {
        run: '6753',
        viewers: [{ label: 'only viewer', apiBaseUrl: 'http://third.test/api' }],
        stop: vi.fn(),
      },
    ];

    expect(
      databaseQuerySources(handles).map(source => ({ id: source.id, label: source.label }))
    ).toEqual([
      { id: '6647#1', label: 'db 6647 · first viewer' },
      { id: '6647#2', label: 'db 6647 · second viewer' },
      { id: '6753', label: 'db 6753' },
    ]);
  });

  it('starts database viewers and always stops them after use', async () => {
    const handle: QuentOpenDbHandle = {
      run: '6647',
      viewers: [{ label: 'viewer', apiBaseUrl: 'http://viewer.test/api' }],
      stop: vi.fn(),
    };
    const runtime: QuerySourceRuntime = {
      launch: vi.fn().mockResolvedValue([handle]),
      stop: vi.fn().mockResolvedValue(undefined),
    };
    const input = vi.fn();

    await expect(
      withQuerySources(
        { 'db-run': ['6647'], json: true },
        input,
        async session => session.sources.map(source => source.id),
        runtime
      )
    ).resolves.toEqual(['6647']);
    expect(input).not.toHaveBeenCalled();
    expect(runtime.launch).toHaveBeenCalledWith(
      ['6647'],
      expect.objectContaining({ interactive: false })
    );
    expect(runtime.stop).toHaveBeenCalledWith([handle]);
  });
});
