// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it, vi } from 'vitest';
import {
  launchQuentOpenDbRuns,
  parseQuentOpenLine,
  quentOpenDbArguments,
  readyApiBaseFromLine,
  type QuentOpenDbHandle,
  type QuentOpenDbOptions,
} from './quentOpenDb';

const options: QuentOpenDbOptions = {
  apiBaseUrl: 'https://benchmark.test',
  interactive: false,
  token: 'secret-token',
  trust: ['github.com/rapidsai/*'],
  trustAll: false,
};

describe('quent-open database launcher', () => {
  it('parses viewer count and ready URLs', () => {
    expect(parseQuentOpenLine('discovered 3 context(s) -> 1 viewer(s)')).toEqual({
      viewerCount: 1,
    });
    expect(
      parseQuentOpenLine('ready: query-engine — 3 context(s)  http://127.0.0.1:49152/')
    ).toEqual({
      apiBaseUrl: 'http://127.0.0.1:49152/api',
    });
    expect(() => readyApiBaseFromLine('discovered 3 context(s) -> 2 viewer(s)', '6647')).toThrow(
      'produced 2 viewer APIs'
    );
  });

  it('keeps credentials out of spawned arguments', () => {
    const args = quentOpenDbArguments('6647', options);

    expect(args).toContain('6647');
    expect(args).toContain('github.com/rapidsai/*');
    expect(args.join(' ')).not.toContain(options.token);
    expect(args.join(' ')).not.toContain(options.apiBaseUrl);
  });

  it('launches unique database runs serially', async () => {
    const order: string[] = [];
    let active = 0;
    let maximumActive = 0;
    const launch = vi.fn(async (run: string): Promise<QuentOpenDbHandle> => {
      active += 1;
      maximumActive = Math.max(maximumActive, active);
      order.push(`start:${run}`);
      await Promise.resolve();
      order.push(`ready:${run}`);
      active -= 1;
      return { run, apiBaseUrl: `http://${run}/api`, stop: vi.fn() };
    });

    const handles = await launchQuentOpenDbRuns(['6647', '6650', '6647'], options, launch);

    expect(handles.map(handle => handle.run)).toEqual(['6647', '6650']);
    expect(order).toEqual(['start:6647', 'ready:6647', 'start:6650', 'ready:6650']);
    expect(maximumActive).toBe(1);
  });

  it('stops earlier runs when a later launch fails', async () => {
    const stop = vi.fn().mockResolvedValue(undefined);
    const launch = vi
      .fn()
      .mockResolvedValueOnce({
        run: '6647',
        apiBaseUrl: 'http://6647/api',
        stop,
      })
      .mockRejectedValueOnce(new Error('startup failed'));

    await expect(launchQuentOpenDbRuns(['6647', '6650'], options, launch)).rejects.toThrow(
      'startup failed'
    );
    expect(stop).toHaveBeenCalledOnce();
  });
});
