// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createElement } from 'react';
import { render } from 'ink';
import type { QueryTreeChoice, SelectionChoice } from './askSelection';
import { MultiSelectionPrompt } from './inkMultiSelectionPrompt';
import { QueryTreePrompt } from './inkQueryTreePrompt';
import { SelectionPrompt } from './inkSelectionPrompt';
import { TextPrompt } from './inkTextPrompt';

function abortError(signal: AbortSignal): Error {
  return signal.reason instanceof Error
    ? signal.reason
    : new DOMException('The operation was aborted.', 'AbortError');
}

async function waitUntilExit(
  instance: ReturnType<typeof render>,
  signal?: AbortSignal
): Promise<unknown> {
  if (!signal) {
    return instance.waitUntilExit();
  }
  if (signal.aborted) {
    instance.unmount();
    throw abortError(signal);
  }
  let onAbort: () => void;
  const aborted = new Promise<never>((_, reject) => {
    onAbort = () => {
      instance.unmount();
      reject(abortError(signal));
    };
    signal.addEventListener('abort', onAbort, { once: true });
  });
  try {
    return await Promise.race([instance.waitUntilExit(), aborted]);
  } finally {
    signal.removeEventListener('abort', onAbort!);
  }
}

export async function selectWithInk(
  prompt: string,
  choices: readonly SelectionChoice[],
  input: NodeJS.ReadStream,
  output: NodeJS.WriteStream,
  signal?: AbortSignal
): Promise<string> {
  const instance = render(createElement(SelectionPrompt, { prompt, choices }), {
    stdin: input,
    stdout: output,
    stderr: output,
    exitOnCtrlC: false,
    patchConsole: false,
    interactive: true,
  });
  try {
    const selected = await waitUntilExit(instance, signal);
    if (typeof selected !== 'string') {
      throw new Error('Selection ended without a result.');
    }
    return selected;
  } finally {
    instance.clear();
    instance.cleanup();
  }
}

export async function selectFromQueryTreeWithInk(
  prompt: string,
  choices: readonly QueryTreeChoice[],
  multiple: boolean,
  signal?: AbortSignal,
  input: NodeJS.ReadStream = process.stdin,
  output: NodeJS.WriteStream = process.stderr
): Promise<string[]> {
  signal?.throwIfAborted();
  if (!input.isTTY || !output.isTTY) {
    throw new Error(`${prompt} requires an interactive terminal; pass --engine and --query.`);
  }
  const instance = render(createElement(QueryTreePrompt, { prompt, choices, multiple }), {
    stdin: input,
    stdout: output,
    stderr: output,
    exitOnCtrlC: false,
    patchConsole: false,
    interactive: true,
  });
  try {
    const selected = await waitUntilExit(instance, signal);
    if (!Array.isArray(selected) || selected.some(value => typeof value !== 'string')) {
      throw new Error('Query selection ended without a result.');
    }
    return selected;
  } finally {
    instance.clear();
    instance.cleanup();
  }
}

export async function selectManyWithInk(
  prompt: string,
  choices: readonly SelectionChoice[],
  allValue?: string,
  signal?: AbortSignal,
  input: NodeJS.ReadStream = process.stdin,
  output: NodeJS.WriteStream = process.stderr
): Promise<string[]> {
  signal?.throwIfAborted();
  if (!input.isTTY || !output.isTTY) {
    throw new Error(`${prompt} requires an interactive terminal; pass --metric.`);
  }
  const instance = render(createElement(MultiSelectionPrompt, { prompt, choices, allValue }), {
    stdin: input,
    stdout: output,
    stderr: output,
    exitOnCtrlC: false,
    patchConsole: false,
    interactive: true,
  });
  try {
    const selected = await waitUntilExit(instance, signal);
    if (!Array.isArray(selected) || selected.some(value => typeof value !== 'string')) {
      throw new Error('Metric selection ended without a result.');
    }
    return selected;
  } finally {
    instance.clear();
    instance.cleanup();
  }
}

export async function inputWithInk(
  prompt: string,
  placeholder?: string,
  signal?: AbortSignal,
  input: NodeJS.ReadStream = process.stdin,
  output: NodeJS.WriteStream = process.stderr
): Promise<string> {
  signal?.throwIfAborted();
  const instance = render(createElement(TextPrompt, { prompt, placeholder }), {
    stdin: input,
    stdout: output,
    stderr: output,
    exitOnCtrlC: false,
    patchConsole: false,
    interactive: true,
  });
  try {
    const value = await waitUntilExit(instance, signal);
    if (typeof value !== 'string') {
      throw new Error('Text input ended without a result.');
    }
    return value;
  } finally {
    instance.clear();
    instance.cleanup();
  }
}
