// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createElement } from 'react';
import { render } from 'ink';
import type { SelectionChoice } from './askSelection';
import { SelectionPrompt } from './inkSelectionPrompt';

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
