// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseArgs } from 'node:util';
import type { QuestionCliValues, RegisteredQuestion } from './questions/question.types';
import type { AskCommand } from './askCommand.types';
import { createTerminalSelector, resolveAskSelections } from './askSelection';
import { machineResult, serializeMachineOutput } from './machineOutput';
import {
  inputDatabaseRunIds,
  optionalId,
  querySourceOptions,
  querySourceUsage,
  selectQuerySource,
  withQuerySources,
} from './querySources';

function usage(question: RegisteredQuestion): string {
  return `Usage:
  pnpm ask ${question.metadata.id} [--engine ID] [--query ID] [options]

Options:
  --engine ID            Engine ID (select from API results when omitted)
  --query ID             Query ID (select from API results when omitted)
  --operator IDS         Comma-separated operator IDs
  --source ID            Query source (select when multiple are available)
${querySourceUsage}
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
          ...querySourceOptions,
          base: { type: 'string' },
          engine: { type: 'string' },
          json: { type: 'boolean' },
          operator: { type: 'string' },
          query: { type: 'string' },
          source: { type: 'string' },
        },
      });
      const questionValues: QuestionCliValues = {
        base: values.base,
        engine: values.engine,
        json: values.json,
        operator: values.operator,
        query: values.query,
      };
      await withQuerySources(values, inputDatabaseRunIds, async ({ sources, databaseMode }) => {
        const select = createTerminalSelector();
        const source = await selectQuerySource(
          sources,
          optionalId(values.source),
          select,
          'Select the query source'
        );
        const requireResource = question.metadata.parameters.some(
          parameter => parameter.name === 'resource' && parameter.required
        );
        const selections = await resolveAskSelections({
          values: questionValues,
          api: source.api,
          select,
          requireResource,
        });
        const result = await question.run(
          {
            engineId: selections.engineId,
            queryId: selections.queryId,
            ...(values.base ? { appBaseUrl: values.base } : {}),
            queryBundle: selections.queryBundle,
            api: { fetchEntityList: source.api.fetchEntityList },
          },
          selections.values
        );
        const output = databaseMode
          ? { ...result, source: { id: source.id, label: source.label } }
          : result;
        process.stdout.write(
          values.json
            ? serializeMachineOutput(machineResult(question.metadata.id, output))
            : `${databaseMode ? `Source: ${source.label}\n` : ''}${question.formatHuman(result)}\n`
        );
      });
    },
  };
}
