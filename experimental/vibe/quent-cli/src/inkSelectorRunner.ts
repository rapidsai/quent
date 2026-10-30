// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createElement } from 'react';
import { render } from 'ink';
import type { QueryTreeChoice, SelectionChoice } from './askSelection';
import { MultiSelectionPrompt } from './inkMultiSelectionPrompt';
import { QueryTreePrompt } from './inkQueryTreePrompt';
import { SelectionPrompt } from './inkSelectionPrompt';
import { TextPrompt } from './inkTextPrompt';

export async function selectWithInk(
  prompt: string,
  choices: readonly SelectionChoice[],
  input: NodeJS.ReadStream,
  output: NodeJS.WriteStream
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
    const selected = await instance.waitUntilExit();
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
  input: NodeJS.ReadStream = process.stdin,
  output: NodeJS.WriteStream = process.stderr
): Promise<string[]> {
  const instance = render(createElement(QueryTreePrompt, { prompt, choices, multiple }), {
    stdin: input,
    stdout: output,
    stderr: output,
    exitOnCtrlC: false,
    patchConsole: false,
    interactive: true,
  });
  try {
    const selected = await instance.waitUntilExit();
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
  input: NodeJS.ReadStream = process.stdin,
  output: NodeJS.WriteStream = process.stderr
): Promise<string[]> {
  const instance = render(createElement(MultiSelectionPrompt, { prompt, choices, allValue }), {
    stdin: input,
    stdout: output,
    stderr: output,
    exitOnCtrlC: false,
    patchConsole: false,
    interactive: true,
  });
  try {
    const selected = await instance.waitUntilExit();
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
  input: NodeJS.ReadStream = process.stdin,
  output: NodeJS.WriteStream = process.stderr
): Promise<string> {
  const instance = render(createElement(TextPrompt, { prompt, placeholder }), {
    stdin: input,
    stdout: output,
    stderr: output,
    exitOnCtrlC: false,
    patchConsole: false,
    interactive: true,
  });
  try {
    const value = await instance.waitUntilExit();
    if (typeof value !== 'string') {
      throw new Error('Text input ended without a result.');
    }
    return value;
  } finally {
    instance.clear();
    instance.cleanup();
  }
}
