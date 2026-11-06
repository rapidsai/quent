// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseArgs } from 'node:util';
import type { AskCommand } from './askCommand.types';
import { createNonInteractiveSelector, createTerminalSelector, selectEngine } from './askSelection';
import { machineResult, serializeMachineOutput } from './machineOutput';
import {
  findQuerySource,
  inputDatabaseRunIds,
  optionalId,
  querySourceOptions,
  querySourceUsage,
  selectQuerySource,
  withQuerySources,
} from './querySources';

type NamedEntity = { id: string; instance_name: string | null };

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
  explanation: 'Lists engines available from local or database-backed Quent APIs.',
  usage: `Usage:
  pnpm ask engines [options]

Options:
  --source ID             Limit results to one query source
${querySourceUsage}
  --json                  Emit versioned JSON`,
  async run(args) {
    const { values } = parseArgs({
      args,
      allowPositionals: false,
      options: {
        ...querySourceOptions,
        json: { type: 'boolean' },
        source: { type: 'string' },
      },
    });
    await withQuerySources(values, inputDatabaseRunIds, async ({ sources, databaseMode }) => {
      const requestedSource = optionalId(values.source);
      const selectedSources = requestedSource
        ? [findQuerySource(sources, requestedSource)]
        : sources;
      const catalogs = await Promise.all(
        selectedSources.map(async source => ({
          source,
          engines: sorted((await source.api.fetchListEngines()).items),
        }))
      );
      const data =
        databaseMode || selectedSources.length > 1
          ? catalogs.flatMap(({ source, engines }) =>
              engines.map(engine => ({
                sourceId: source.id,
                sourceLabel: source.label,
                ...engine,
              }))
            )
          : (catalogs[0]?.engines ?? []);
      const human = catalogs
        .map(({ source, engines }) =>
          formatList(
            databaseMode || selectedSources.length > 1 ? `Engines from ${source.label}` : 'Engines',
            engines
          )
        )
        .join('\n\n');
      process.stdout.write(
        values.json ? serializeMachineOutput(machineResult('engines', data)) : `${human}\n`
      );
    });
  },
};

export const queriesCommand: AskCommand = {
  id: 'queries',
  explanation: 'Lists queries for an engine from a local or database-backed Quent API.',
  usage: `Usage:
  pnpm ask queries [options]

Options:
  --engine ID             Engine ID (required with --json)
  --source ID             Query source (required with --json when multiple are available)
${querySourceUsage}
  --json                  Emit versioned JSON; never open interactive prompts`,
  async run(args) {
    const { values } = parseArgs({
      args,
      allowPositionals: false,
      options: {
        ...querySourceOptions,
        engine: { type: 'string' },
        json: { type: 'boolean' },
        source: { type: 'string' },
      },
    });
    if (values.json && !values.engine) {
      throw new Error('JSON mode requires explicit --engine. Run "pnpm ask engines --json" first.');
    }
    await withQuerySources(values, inputDatabaseRunIds, async ({ sources, databaseMode }) => {
      const select = selector(values.json);
      const source = await selectQuerySource(sources, optionalId(values.source), select);
      const engineId =
        values.engine ?? (await selectEngine(source.api, select, 'Select an engine'));
      const queries = sorted((await source.api.fetchListQueries(engineId)).items);
      const data = {
        ...(databaseMode ? { sourceId: source.id, sourceLabel: source.label } : {}),
        engineId,
        queries,
      };
      process.stdout.write(
        values.json
          ? serializeMachineOutput(machineResult('queries', data))
          : `${formatList(
              `Queries for ${databaseMode ? `${source.label} · ` : ''}${engineId}`,
              queries
            )}\n`
      );
    });
  },
};

export const discoveryCommands = [enginesCommand, queriesCommand] as const;
