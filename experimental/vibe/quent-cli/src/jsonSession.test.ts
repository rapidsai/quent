// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Readable } from 'node:stream';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { getAskCommand } from './askCommands';
import { runJsonSession, runJsonSessionRequests } from './jsonSession';

async function* requestLines(...lines: string[]): AsyncGenerator<string> {
  yield* lines;
}

describe('JSON command session', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('injects JSON mode, reports request errors, and continues', async () => {
    const enginesRun = vi
      .spyOn(getAskCommand('engines'), 'run')
      .mockRejectedValueOnce(new Error('temporary failure'))
      .mockResolvedValueOnce();
    const write = vi.fn();

    await runJsonSessionRequests(
      requestLines(
        '{"command":"engines","args":["--source","6647"]}',
        '{"command":"engines"}',
        '{"command":"exit"}'
      ),
      write
    );

    expect(enginesRun).toHaveBeenNthCalledWith(1, ['--source', '6647', '--json']);
    expect(enginesRun).toHaveBeenNthCalledWith(2, ['--json']);
    expect(JSON.parse(write.mock.calls[0]![0])).toMatchObject({
      command: 'engines',
      error: { message: 'temporary failure' },
    });
    expect(write.mock.calls[0]![0]).not.toContain('\n ');
  });

  it('keeps malformed requests inside the machine protocol', async () => {
    const enginesRun = vi.spyOn(getAskCommand('engines'), 'run').mockResolvedValue();
    const write = vi.fn();

    await runJsonSessionRequests(requestLines('not JSON', '{"command":"exit"}'), write);

    expect(enginesRun).not.toHaveBeenCalled();
    expect(JSON.parse(write.mock.calls[0]![0])).toMatchObject({
      command: null,
      error: { message: expect.stringContaining('JSON') },
    });
  });

  it('runs a local session until stdin closes', async () => {
    const enginesRun = vi.spyOn(getAskCommand('engines'), 'run').mockResolvedValue();
    const input = Readable.from(['{"command":"engines"}\n']);

    await runJsonSession(['--json'], input);

    expect(enginesRun).toHaveBeenCalledWith(['--json']);
  });

  it('requires explicit database runs', async () => {
    await expect(runJsonSession(['--db', '--json'], Readable.from([]))).rejects.toThrow(
      'requires explicit --db-run'
    );
  });
});
