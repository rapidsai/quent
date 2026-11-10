// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { spawn, type ChildProcess } from 'node:child_process';
import { once } from 'node:events';
import path from 'node:path';

const REPOSITORY_ROOT = path.resolve(import.meta.dirname, '../../../..');
const MAX_CAPTURED_OUTPUT = 64 * 1024;
const STOP_TIMEOUT_MS = 5_000;

export interface QuentOpenDbOptions {
  apiBaseUrl?: string;
  interactive: boolean;
  signal?: AbortSignal;
  token?: string;
  trust: readonly string[];
  trustAll: boolean;
}

export interface QuentOpenDbHandle {
  run: string;
  viewers: QuentOpenDbViewer[];
  stop(): Promise<void>;
}

export interface QuentOpenDbViewer {
  label: string;
  apiBaseUrl: string;
}

export interface QuentOpenReadyState {
  viewerCount?: number;
  viewer?: QuentOpenDbViewer;
}

export function parseQuentOpenLine(line: string): QuentOpenReadyState {
  const discoveredMatch = line.match(/discovered \d+ context\(s\) -> (\d+) viewer\(s\)/u);
  const servingMatch = line.match(/\bserving (\d+) viewer\(s\)/u);
  const readyMatch = line.match(/\bready:\s*(.+?)\s+(https?:\/\/\S+)\s*$/u);
  return {
    ...(servingMatch || discoveredMatch
      ? { viewerCount: Number((servingMatch ?? discoveredMatch)![1]) }
      : {}),
    ...(readyMatch
      ? {
          viewer: {
            label: readyMatch[1]!.trim(),
            apiBaseUrl: `${readyMatch[2]!.replace(/\/+$/u, '')}/api`,
          },
        }
      : {}),
  };
}

export function createQuentOpenReadyTracker(): {
  consume(line: string): QuentOpenDbViewer[] | undefined;
} {
  let expectedViewerCount: number | undefined;
  const viewers: QuentOpenDbViewer[] = [];
  return {
    consume(line) {
      const state = parseQuentOpenLine(line);
      expectedViewerCount = state.viewerCount ?? expectedViewerCount;
      if (state.viewer) {
        viewers.push(state.viewer);
      }
      if (
        expectedViewerCount !== undefined &&
        expectedViewerCount > 0 &&
        viewers.length === expectedViewerCount
      ) {
        return [...viewers];
      }
      return undefined;
    },
  };
}

export function quentOpenDbArguments(run: string, options: QuentOpenDbOptions): string[] {
  return [
    'run',
    'cargo',
    'run',
    '-p',
    'quent-open',
    '--features',
    'db',
    '--',
    '--no-browser',
    ...options.trust.flatMap(remote => ['--trust', remote]),
    ...(options.trustAll ? ['--trust-all'] : []),
    'db',
    run,
  ];
}

function childEnvironment(options: QuentOpenDbOptions): NodeJS.ProcessEnv {
  return {
    ...process.env,
    ...(options.apiBaseUrl ? { QUENT_OPEN_API_BASE_URL: options.apiBaseUrl } : {}),
    ...(options.token ? { QUENT_OPEN_TOKEN: options.token } : {}),
  };
}

function outputTail(output: string): string {
  return output.split('\n').slice(-20).join('\n').trim();
}

function captureOutput(output: string, chunk: string): string {
  return `${output}${chunk}`.slice(-MAX_CAPTURED_OUTPUT);
}

function abortError(signal: AbortSignal): Error {
  return signal.reason instanceof Error
    ? signal.reason
    : new DOMException('The operation was aborted.', 'AbortError');
}

async function stopChild(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  child.kill('SIGINT');
  let timeout: NodeJS.Timeout | undefined;
  try {
    await Promise.race([
      once(child, 'exit'),
      new Promise(resolve => {
        timeout = setTimeout(resolve, STOP_TIMEOUT_MS);
        timeout.unref();
      }),
    ]);
  } finally {
    clearTimeout(timeout);
  }
  if (child.exitCode === null && child.signalCode === null) {
    child.kill('SIGKILL');
    await once(child, 'exit');
  }
}

export async function launchQuentOpenDb(
  run: string,
  options: QuentOpenDbOptions
): Promise<QuentOpenDbHandle> {
  options.signal?.throwIfAborted();
  const child = spawn('pixi', quentOpenDbArguments(run, options), {
    cwd: REPOSITORY_ROOT,
    env: childEnvironment(options),
    stdio: [options.interactive ? 'inherit' : 'ignore', 'pipe', 'pipe'],
  });
  let output = '';
  let lineBuffer = '';
  let isReady = false;
  const readyTracker = createQuentOpenReadyTracker();
  child.stderr?.on('data', chunk => {
    const text = String(chunk);
    if (!isReady) {
      output = captureOutput(output, text);
    }
    process.stderr.write(text);
  });

  let onAbort: (() => void) | undefined;
  const ready = new Promise<QuentOpenDbViewer[]>((resolve, reject) => {
    if (options.signal) {
      onAbort = () => {
        void stopChild(child);
        reject(abortError(options.signal!));
      };
      options.signal.addEventListener('abort', onAbort, { once: true });
      if (options.signal.aborted) {
        onAbort();
      }
    }
    child.once('error', reject);
    child.once('exit', (code, signal) => {
      reject(
        new Error(
          `quent-open for database run "${run}" exited before becoming ready (${signal ?? code ?? 'unknown'}).${outputTail(output) ? `\n${outputTail(output)}` : ''}`
        )
      );
    });
    child.stdout?.on('data', chunk => {
      const text = String(chunk);
      process.stderr.write(text);
      if (isReady) {
        return;
      }
      output = captureOutput(output, text);
      lineBuffer += text;
      const lines = lineBuffer.split('\n');
      lineBuffer = lines.pop() ?? '';
      for (const line of lines) {
        const viewers = readyTracker.consume(line);
        if (viewers) {
          isReady = true;
          resolve(viewers);
          return;
        }
      }
    });
  });

  try {
    const viewers = await ready;
    return {
      run,
      viewers,
      stop: () => stopChild(child),
    };
  } catch (error) {
    await stopChild(child);
    throw error;
  } finally {
    if (options.signal && onAbort) {
      options.signal.removeEventListener('abort', onAbort);
    }
  }
}

export async function launchQuentOpenDbRuns(
  runs: readonly string[],
  options: QuentOpenDbOptions,
  launch: typeof launchQuentOpenDb = launchQuentOpenDb
): Promise<QuentOpenDbHandle[]> {
  const handles: QuentOpenDbHandle[] = [];
  try {
    for (const run of [...new Set(runs)]) {
      options.signal?.throwIfAborted();
      handles.push(await launch(run, options));
    }
    return handles;
  } catch (error) {
    await stopQuentOpenDbRuns(handles);
    throw error;
  }
}

export async function stopQuentOpenDbRuns(handles: readonly QuentOpenDbHandle[]): Promise<void> {
  await Promise.allSettled([...handles].reverse().map(handle => handle.stop()));
}
