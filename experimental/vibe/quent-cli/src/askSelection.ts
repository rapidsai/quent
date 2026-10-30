// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Readable, Writable } from 'node:stream';
import type { ApiClient } from '@quent/client';
import type { EntityRef, QueryBundle } from '@quent/utils';
import type { QuestionCliValues } from './questions/question.types';
import { selectFromQueryTreeWithInk, selectWithInk } from './inkSelectorRunner';

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

export interface QuerySelection {
  engineId: string;
  queryId: string;
}

export interface QueryTreeChoice extends SelectionChoice {
  sourceId?: string;
  engineId: string;
  engineLabel: string;
  queryId: string;
}

export type SelectFromQueryTree = (
  prompt: string,
  choices: readonly QueryTreeChoice[],
  multiple: boolean
) => Promise<string[]>;

export type DiscoveryApi = Pick<
  ApiClient,
  'fetchListEngines' | 'fetchListQueries' | 'fetchQueryBundle'
>;

interface ResolveAskSelectionsOptions {
  values: QuestionCliValues;
  api: DiscoveryApi;
  select: SelectFromList;
  selectTree?: SelectFromQueryTree;
  requireResource: boolean;
}

export interface ResolvedAskSelections {
  engineId: string;
  queryId: string;
  queryBundle: QueryBundle<EntityRef>;
  values: QuestionCliValues;
}

function optionalId(value: string | boolean | undefined): string | undefined {
  return typeof value === 'string' && value.trim() !== '' ? value.trim() : undefined;
}

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

export function querySelectionKey(engineId: string, queryId: string): string {
  return `${engineId}\0${queryId}`;
}

export async function discoverQueryTreeChoices(
  api: DiscoveryApi,
  engineIds?: ReadonlySet<string>
): Promise<QueryTreeChoice[]> {
  const { items: engines } = await api.fetchListEngines();
  const engineBranches = await Promise.all(
    engines
      .filter(engine => !engineIds || engineIds.has(engine.id))
      .map(async engine => ({ engine, queries: (await api.fetchListQueries(engine.id)).items }))
  );
  return engineBranches
    .flatMap(({ engine, queries }) =>
      queries.map(query => ({
        value: querySelectionKey(engine.id, query.id),
        label: label(query.instance_name, query.id),
        engineId: engine.id,
        engineLabel: label(engine.instance_name, engine.id),
        queryId: query.id,
      }))
    )
    .sort(
      (left, right) =>
        left.engineLabel.localeCompare(right.engineLabel) ||
        left.label.localeCompare(right.label) ||
        left.value.localeCompare(right.value)
    );
}

export async function selectQueriesFromTree(
  api: DiscoveryApi,
  select: SelectFromQueryTree,
  prompt: string,
  multiple: boolean,
  excluded: ReadonlySet<string> = new Set(),
  discoveredChoices?: readonly QueryTreeChoice[]
): Promise<QuerySelection[]> {
  const choices = (discoveredChoices ?? (await discoverQueryTreeChoices(api))).filter(
    choice => !excluded.has(choice.value)
  );
  if (choices.length === 0) {
    throw new Error('The Quent API returned no available queries.');
  }
  const selected = await select(prompt, choices, multiple);
  const byValue = new Map(choices.map(choice => [choice.value, choice]));
  return selected.map(value => {
    const choice = byValue.get(value);
    if (!choice) {
      throw new Error(`Selected query "${value}" is unavailable.`);
    }
    return { engineId: choice.engineId, queryId: choice.queryId };
  });
}

async function selectResource(
  bundle: QueryBundle<EntityRef>,
  select: SelectFromList
): Promise<string> {
  const choices = stableChoices(
    Object.values(bundle.entities.resources).flatMap(resource =>
      resource
        ? [
            {
              value: resource.id,
              label: `${resource.instance_name} (${resource.type_name}, ${resource.id})`,
            },
          ]
        : []
    )
  );
  if (choices.length === 0) {
    throw new Error(`Query "${bundle.query_id}" contains no available resources.`);
  }
  return select('Select a resource', choices);
}

export async function resolveAskSelections({
  values,
  api,
  select,
  selectTree = selectFromQueryTreeWithInk,
  requireResource,
}: ResolveAskSelectionsOptions): Promise<ResolvedAskSelections> {
  const requestedEngineId = optionalId(values.engine);
  const requestedQueryId = optionalId(values.query);
  let engineId = requestedEngineId;
  let queryId = requestedQueryId;
  if (!engineId || !queryId) {
    const choices = (
      await discoverQueryTreeChoices(api, engineId ? new Set([engineId]) : undefined)
    ).filter(choice => !queryId || choice.queryId === queryId);
    const [selection] = await selectQueriesFromTree(
      api,
      selectTree,
      'Select a query',
      false,
      new Set(),
      choices
    );
    engineId = selection.engineId;
    queryId = selection.queryId;
  }
  const queryBundle = await api.fetchQueryBundle(engineId, queryId);
  const resourceId =
    optionalId(values.resource) ??
    (requireResource ? await selectResource(queryBundle, select) : undefined);

  return {
    engineId,
    queryId,
    queryBundle,
    values: {
      ...values,
      engine: engineId,
      query: queryId,
      ...(resourceId ? { resource: resourceId } : {}),
    },
  };
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
