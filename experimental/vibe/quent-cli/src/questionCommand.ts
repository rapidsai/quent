// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseArgs } from 'node:util';
import {
  fetchEntityList,
  fetchListEngines,
  fetchListQueries,
  fetchQueryBundle,
  setApiBaseUrl,
  type ApiClient,
} from '@quent/client';
import type { RegisteredQuestion } from './questions/question.types';
import type { AskCommand } from './askCommand.types';
import { createTerminalSelector, resolveAskSelections } from './askSelection';
import { machineResult, serializeMachineOutput } from './machineOutput';

function usage(question: RegisteredQuestion): string {
  return `Usage:
  pnpm ask ${question.metadata.id} [--engine ID] [--query ID] [--resource ID] [options]

Options:
  --engine ID            Engine ID (select from API results when omitted)
  --query ID             Query ID (select from API results when omitted)
  --resource ID          Resource ID (select from the query bundle when omitted)
  --entity-type NAME     Restrict to one declared FSM entity type
  --operator IDS         Comma-separated operator IDs
  --start SECONDS        Query-relative window start (default: 0)
  --end SECONDS          Query-relative window end (default: query duration)
  --limit COUNT          Maximum results, 1-500 (default: 10)
  --api-base URL         API base (default: QUENT_API_BASE_URL or http://localhost:8080/api)
  --base URL             Quent UI base for an absolute deep link
  --json                 Emit versioned JSON output (selection may still be interactive)`;
}

export function createQuestionCommand(question: RegisteredQuestion): AskCommand {
  return {
    id: question.metadata.id,
    explanation: question.metadata.explanation,
    usage: usage(question),
    async run(args) {
      const { values } = parseArgs({
        args,
        allowPositionals: false,
        options: {
          'api-base': { type: 'string' },
          base: { type: 'string' },
          end: { type: 'string' },
          engine: { type: 'string' },
          'entity-type': { type: 'string' },
          json: { type: 'boolean' },
          limit: { type: 'string' },
          operator: { type: 'string' },
          query: { type: 'string' },
          resource: { type: 'string' },
          start: { type: 'string' },
        },
      });
      const apiBase =
        values['api-base'] ?? process.env.QUENT_API_BASE_URL ?? 'http://localhost:8080/api';
      setApiBaseUrl(apiBase.replace(/\/+$/u, ''));

      const requireResource = question.metadata.parameters.some(
        parameter => parameter.name === 'resource' && parameter.required
      );
      const selections = await resolveAskSelections({
        values,
        api: {
          fetchListEngines,
          fetchListQueries,
          fetchQueryBundle,
        },
        select: createTerminalSelector(),
        requireResource,
      });
      const result = await question.run(
        {
          engineId: selections.engineId,
          queryId: selections.queryId,
          ...(values.base ? { appBaseUrl: values.base } : {}),
          queryBundle: selections.queryBundle,
          api: {
            fetchEntityList: (engineId, request): ReturnType<ApiClient['fetchEntityList']> =>
              fetchEntityList(engineId, request),
          },
        },
        selections.values
      );
      process.stdout.write(
        values.json
          ? serializeMachineOutput(machineResult(question.metadata.id, result))
          : `${question.formatHuman(result)}\n`
      );
    },
  };
}
