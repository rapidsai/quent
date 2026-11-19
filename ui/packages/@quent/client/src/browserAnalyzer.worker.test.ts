// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import type { BrowserAnalyzerWorkerReply, BrowserAnalyzerWorkerRequest } from './browserAnalyzer';

type WorkerHandler = (event: MessageEvent<BrowserAnalyzerWorkerRequest>) => Promise<void>;

const postMessage = vi.spyOn(self, 'postMessage').mockImplementation(() => {});
const fetchSpy = vi.spyOn(globalThis, 'fetch');

beforeAll(async () => {
  // Importing the module installs `self.onmessage`.
  await import('./browserAnalyzer.worker');
});

afterEach(() => {
  postMessage.mockClear();
  fetchSpy.mockClear();
});

/** Deliver a request to the worker and return the reply it posted. */
async function send(request: BrowserAnalyzerWorkerRequest): Promise<BrowserAnalyzerWorkerReply> {
  const handler = self.onmessage as unknown as WorkerHandler;
  await handler({ data: request } as MessageEvent<BrowserAnalyzerWorkerRequest>);
  expect(postMessage).toHaveBeenCalledTimes(1);
  return postMessage.mock.calls[0]![0] as BrowserAnalyzerWorkerReply;
}

describe('browser analyzer worker init', () => {
  it.each([
    ['another origin', 'https://example.invalid/analyzer.js'],
    ['a data: URL', 'data:text/javascript,export default 1'],
  ])('rejects a module hosted on %s without loading it', async (_name, moduleUrl) => {
    const reply = await send({ id: 7, type: 'init', moduleUrl, dataUrl: 'trace.quent-browser' });

    expect(reply).toEqual({
      id: 7,
      error: {
        code: 'invalid-request',
        message: 'browser analyzer modules must be hosted on the Quent UI origin',
      },
    });
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
