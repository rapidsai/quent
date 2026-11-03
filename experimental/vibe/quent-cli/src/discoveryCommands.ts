// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseArgs } from 'node:util';
import { fetchListEngines, fetchListQueries, setApiBaseUrl } from '@quent/client';
import type { AskCommand } from './askCommand.types';
import {
  createNonInteractiveSelector,
  createTerminalSelector,
  selectEngine,
  type DiscoveryApi,
} from './askSelection';
import { machineResult, serializeMachineOutput } from './machineOutput';

type NamedEntity = { id: string; instance_name: string | null };

const api: DiscoveryApi = {
  fetchListEngines,
  fetchListQueries,
};

function configureApi(apiBase: string | undefined): void {
  const resolved = apiBase ?? process.env.QUENT_API_BASE_URL ?? 'http://localhost:8080/api';
  setApiBaseUrl(resolved.replace(/\/+$/u, ''));
}

function sorted<T extends NamedEntity>(items: T[]): T[] {
  return items.sort(
    (left, right) =>
      (left.instance_name ?? left.id).localeCompare(right.instance_name ?? right.id) ||
      left.id.localeCompare(right.id)
  );
}

function formatList(title: string, items: NamedEntity[]): string {
  return [
    title,
    ...items.map(item =>
      item.instance_name && item.instance_name !== item.id
        ? `  ${item.instance_name} (${item.id})`
        : `  ${item.id}`
    ),
  ].join('\n');
}

function selector(json: boolean | undefined) {
  return json ? createNonInteractiveSelector() : createTerminalSelector();
}

export const enginesCommand: AskCommand = {
  id: 'engines',
  explanation: 'Lists engines available from the Quent API.',
  usage: `Usage:
  pnpm ask engines [--api-base URL] [--json]

Options:
  --api-base URL          API base (default: QUENT_API_BASE_URL or http://localhost:8080/api)
  --json                  Emit versioned JSON`,
  async run(args) {
    const { values } = parseArgs({
      args,
      allowPositionals: false,
      options: {
        'api-base': { type: 'string' },
        json: { type: 'boolean' },
      },
    });
    configureApi(values['api-base']);
    const engines = sorted((await api.fetchListEngines()).items);
    process.stdout.write(
      values.json
        ? serializeMachineOutput(machineResult('engines', engines))
        : `${formatList('Engines', engines)}\n`
    );
  },
};

export const queriesCommand: AskCommand = {
  id: 'queries',
  explanation: 'Lists queries for an engine.',
  usage: `Usage:
  pnpm ask queries [--engine ID] [--api-base URL] [--json]

Options:
  --engine ID             Engine ID (required with --json)
  --api-base URL          API base (default: QUENT_API_BASE_URL or http://localhost:8080/api)
  --json                  Emit versioned JSON; never open interactive prompts`,
  async run(args) {
    const { values } = parseArgs({
      args,
      allowPositionals: false,
      options: {
        'api-base': { type: 'string' },
        engine: { type: 'string' },
        json: { type: 'boolean' },
      },
    });
    configureApi(values['api-base']);
    if (values.json && !values.engine) {
      throw new Error('JSON mode requires explicit --engine. Run "pnpm ask engines --json" first.');
    }
    const select = selector(values.json);
    const engineId = values.engine ?? (await selectEngine(api, select, 'Select an engine'));
    const queries = sorted((await api.fetchListQueries(engineId)).items);
    const data = { engineId, queries };
    process.stdout.write(
      values.json
        ? serializeMachineOutput(machineResult('queries', data))
        : `${formatList(`Queries for ${engineId}`, queries)}\n`
    );
  },
};

export const discoveryCommands = [enginesCommand, queriesCommand] as const;
