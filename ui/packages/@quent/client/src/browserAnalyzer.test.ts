// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  BrowserAnalyzerError,
  browserAnalyzerErrorReply,
  createBrowserAnalyzerClient,
  type BrowserAnalyzerWorkerReply,
  type BrowserAnalyzerWorkerRequest,
} from './browserAnalyzer';

class TestWorker {
  onmessage: ((event: MessageEvent<BrowserAnalyzerWorkerReply>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  onmessageerror: ((event: MessageEvent) => void) | null = null;
  requests: BrowserAnalyzerWorkerRequest[] = [];

  postMessage(request: BrowserAnalyzerWorkerRequest) {
    this.requests.push(request);
    queueMicrotask(() => {
      if (request.type === 'init') {
        this.reply({ id: request.id, response: '{}' });
      } else if (request.operation === 'listEngines') {
        this.reply({
          id: request.id,
          response:
            '{"items":[{"id":"engine","start_time_unix_ns":9007199254740993}],"initial_group_by_attribute":null}',
        });
      } else if (request.operation === 'dataFlow') {
        this.reply({
          id: request.id,
          error: { code: 'unsupported', message: 'data flow is unavailable' },
        });
      }
    });
  }

  terminate() {}

  private reply(data: BrowserAnalyzerWorkerReply) {
    this.onmessage?.({ data } as MessageEvent<BrowserAnalyzerWorkerReply>);
  }
}

describe('createBrowserAnalyzerClient', () => {
  it('initializes the worker and preserves BigInt response fields', async () => {
    const worker = new TestWorker();
    const client = await createBrowserAnalyzerClient(worker, {
      moduleUrl: 'analyzer.js',
      dataUrl: 'trace.quent-browser',
    });
    const engines = await client.fetchListEngines();

    expect(worker.requests[0]).toMatchObject({
      type: 'init',
      moduleUrl: 'analyzer.js',
      dataUrl: 'trace.quent-browser',
    });
    expect(engines.items[0]?.start_time_unix_ns).toBe(9007199254740993n);
  });

  it('maps unsupported data flow to optional absence', async () => {
    const worker = new TestWorker();
    const client = await createBrowserAnalyzerClient(worker, {
      moduleUrl: 'analyzer.js',
      dataUrl: 'trace.quent-browser',
    });

    await expect(
      client.fetchDataFlow('engine', 'query', {
        start: 0,
        end: 1,
        num_bins: 1,
      })
    ).resolves.toBeNull();
  });

  it('exposes typed worker failures', () => {
    expect(new BrowserAnalyzerError('not-found', 'missing')).toMatchObject({
      code: 'not-found',
      message: 'missing',
    });
    expect(browserAnalyzerErrorReply({ code: 'unsupported', message: 'no data flow' })).toEqual({
      code: 'unsupported',
      message: 'no data flow',
    });
    expect(
      browserAnalyzerErrorReply({
        code: 'invalid-bundle',
        message: 'truncated',
      })
    ).toEqual({
      code: 'invalid-bundle',
      message: 'truncated',
    });
    expect(
      browserAnalyzerErrorReply({
        code: 'incompatible-bundle',
        message: 'schema mismatch',
      })
    ).toEqual({
      code: 'incompatible-bundle',
      message: 'schema mismatch',
    });
    expect(browserAnalyzerErrorReply('failure echoed QUENT_NOT_FOUND:bundle text')).toEqual({
      code: 'analyzer-failure',
      message: 'failure echoed QUENT_NOT_FOUND:bundle text',
    });
  });

  it('passes an existing BrowserAnalyzerError through unchanged', () => {
    expect(
      browserAnalyzerErrorReply(new BrowserAnalyzerError('invalid-request', 'bad operation'))
    ).toEqual({ code: 'invalid-request', message: 'bad operation' });
  });
});
