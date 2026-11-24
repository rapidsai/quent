// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import { runAskCommandLoop, shouldRunCommandLoop } from './askLoop';
import { getAskCommand } from './askCommands';
import type { SelectFromList } from './askSelection';

describe('ask command loop', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('runs subsequent commands without repeating the initial arguments', async () => {
    const enginesRun = vi.spyOn(getAskCommand('engines'), 'run').mockResolvedValue();
    const queriesRun = vi.spyOn(getAskCommand('queries'), 'run').mockResolvedValue();
    const select = vi
      .fn<SelectFromList>()
      .mockResolvedValueOnce('queries')
      .mockResolvedValueOnce('__exit');

    await runAskCommandLoop('engines', ['--db-run', '6647'], select);

    expect(enginesRun).toHaveBeenCalledWith(['--db-run', '6647']);
    expect(queriesRun).toHaveBeenCalledWith([]);
    expect(select).toHaveBeenCalledTimes(2);
    const choices = select.mock.calls[0]?.[1] ?? [];
    expect(choices[choices.length - 1]).toEqual({ value: '__exit', label: 'Exit' });
  });

  it('only repeats interactive database invocations', () => {
    expect(shouldRunCommandLoop(['--db-run', '6647'], true, true)).toBe(true);
    expect(shouldRunCommandLoop(['--db-run=6647'], true, true)).toBe(true);
    expect(shouldRunCommandLoop(['--db'], true, true)).toBe(true);
    expect(shouldRunCommandLoop([], true, true)).toBe(false);
    expect(shouldRunCommandLoop(['--db-run', '6647', '--json'], true, true)).toBe(false);
    expect(shouldRunCommandLoop(['--db-run', '6647'], false, true)).toBe(false);
  });
});
