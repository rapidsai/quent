// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';

const { createWorker, createClient, setClient } = vi.hoisted(() => ({
  createWorker: vi.fn(() => ({ terminate: vi.fn() })),
  createClient: vi.fn(async () => ({})),
  setClient: vi.fn(),
}));

vi.mock('@quent/client', () => ({
  createBrowserAnalyzerWorker: createWorker,
  createBrowserAnalyzerClient: createClient,
  setApiClient: setClient,
}));

import { installBrowserAnalyzerClient } from './browserAnalyzerClient';

const defaults = { modulePath: 'analyzers/test.js', dataPath: 'traces/test.quent-browser' };

function setSearch(search: string) {
  window.history.replaceState(null, '', `/${search}`);
}

describe('installBrowserAnalyzerClient', () => {
  afterEach(() => {
    setSearch('');
    vi.clearAllMocks();
  });

  it('rejects a module hosted on another origin before creating the worker', async () => {
    setSearch('?module=https://example.invalid/analyzer.js');

    await expect(installBrowserAnalyzerClient(defaults)).rejects.toThrow(
      'browser analyzer modules must be hosted on the Quent UI origin'
    );

    expect(createWorker).not.toHaveBeenCalled();
    expect(setClient).not.toHaveBeenCalled();
  });

  it('creates the worker and installs the client for a same-origin module', async () => {
    setSearch('?module=/custom/analyzer.js');

    await installBrowserAnalyzerClient(defaults);

    expect(createWorker).toHaveBeenCalledTimes(1);
    expect(createClient).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ moduleUrl: `${window.location.origin}/custom/analyzer.js` })
    );
    expect(setClient).toHaveBeenCalledTimes(1);
  });
});
