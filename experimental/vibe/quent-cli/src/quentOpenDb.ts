// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { spawn, type ChildProcess } from 'node:child_process';
import { once } from 'node:events';
import path from 'node:path';

const REPOSITORY_ROOT = path.resolve(import.meta.dirname, '../..');
const MAX_CAPTURED_OUTPUT = 64 * 1024;
const STOP_TIMEOUT_MS = 5_000;

export interface QuentOpenDbOptions {
  apiBaseUrl?: string;
  interactive: boolean;
  token?: string;
  trust: readonly string[];
  trustAll: boolean;
}

export interface QuentOpenDbHandle {
  run: string;
  apiBaseUrl: string;
  stop(): Promise<void>;
}

export interface QuentOpenReadyState {
  apiBaseUrl?: string;
  viewerCount?: number;
}

export function parseQuentOpenLine(line: string): QuentOpenReadyState {
  const viewerMatch = line.match(/discovered \d+ context\(s\) -> (\d+) viewer\(s\)/u);
  const readyMatch = line.match(/\bready:.*\s(https?:\/\/\S+)\s*$/u);
  return {
    ...(viewerMatch ? { viewerCount: Number(viewerMatch[1]) } : {}),
    ...(readyMatch ? { apiBaseUrl: `${readyMatch[1]!.replace(/\/+$/u, '')}/api` } : {}),
  };
}

export function readyApiBaseFromLine(line: string, run: string): string | undefined {
  const state = parseQuentOpenLine(line);
  if (state.viewerCount !== undefined && state.viewerCount !== 1) {
    throw new Error(
      `Database run "${run}" produced ${state.viewerCount} viewer APIs; query-diff currently requires exactly one.`
    );
  }
  return state.apiBaseUrl;
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

async function stopChild(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  child.kill('SIGINT');
  await Promise.race([
    once(child, 'exit'),
    new Promise(resolve => {
      setTimeout(resolve, STOP_TIMEOUT_MS);
    }),
  ]);
  if (child.exitCode === null && child.signalCode === null) {
    child.kill('SIGKILL');
    await once(child, 'exit');
  }
}

export async function launchQuentOpenDb(
  run: string,
  options: QuentOpenDbOptions
): Promise<QuentOpenDbHandle> {
  const child = spawn('pixi', quentOpenDbArguments(run, options), {
    cwd: REPOSITORY_ROOT,
    env: childEnvironment(options),
    stdio: [options.interactive ? 'inherit' : 'ignore', 'pipe', 'pipe'],
  });
  let output = '';
  let lineBuffer = '';
  let isReady = false;
  child.stderr?.on('data', chunk => {
    const text = String(chunk);
    if (!isReady) {
      output = captureOutput(output, text);
    }
    process.stderr.write(text);
  });

  const ready = new Promise<string>((resolve, reject) => {
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
        let apiBaseUrl: string | undefined;
        try {
          apiBaseUrl = readyApiBaseFromLine(line, run);
        } catch (error) {
          reject(error);
          return;
        }
        if (apiBaseUrl) {
          isReady = true;
          resolve(apiBaseUrl);
          return;
        }
      }
    });
  });

  try {
    const apiBaseUrl = await ready;
    return {
      run,
      apiBaseUrl,
      stop: () => stopChild(child),
    };
  } catch (error) {
    await stopChild(child);
    throw error;
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
