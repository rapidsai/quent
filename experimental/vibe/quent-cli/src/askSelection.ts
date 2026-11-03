// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Readable, Writable } from 'node:stream';
import type { ApiClient } from '@quent/client';
import { selectWithInk } from './inkSelectorRunner';

type TerminalReadable = Readable & { isTTY?: boolean };
type TerminalWritable = Writable & { isTTY?: boolean };

export interface SelectionChoice {
  value: string;
  label: string;
}

export type SelectFromList = (
  prompt: string,
  choices: readonly SelectionChoice[]
) => Promise<string>;

export type DiscoveryApi = Pick<ApiClient, 'fetchListEngines' | 'fetchListQueries'>;

function label(name: string | null, id: string): string {
  return name && name !== id ? `${name} (${id})` : id;
}

function stableChoices(choices: SelectionChoice[]): SelectionChoice[] {
  return choices.sort(
    (left, right) => left.label.localeCompare(right.label) || left.value.localeCompare(right.value)
  );
}

export async function selectEngine(
  api: DiscoveryApi,
  select: SelectFromList,
  prompt = 'Select an engine'
): Promise<string> {
  const { items: engines } = await api.fetchListEngines();
  const choices = stableChoices(
    engines.map(engine => ({
      value: engine.id,
      label: label(engine.instance_name, engine.id),
    }))
  );
  if (choices.length === 0) {
    throw new Error('The Quent API returned no available engines.');
  }
  return select(prompt, choices);
}

export function createTerminalSelector(
  input: TerminalReadable = process.stdin,
  output: TerminalWritable = process.stderr
): SelectFromList {
  return async (prompt, choices) => {
    if (!input.isTTY || !output.isTTY) {
      throw new Error(`${prompt} requires an interactive terminal; pass the corresponding ID.`);
    }
    return await selectWithInk(
      prompt,
      choices,
      input as NodeJS.ReadStream,
      output as NodeJS.WriteStream
    );
  };
}

export function createNonInteractiveSelector(): SelectFromList {
  return async prompt => {
    throw new Error(
      `${prompt} is required in JSON mode; pass its ID explicitly or use a discovery command.`
    );
  };
}
