// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createInterface } from 'node:readline';
import type { Readable } from 'node:stream';
import { parseArgs } from 'node:util';
import { getAskCommand } from './askCommands';
import { machineError, serializeMachineOutput, withCompactMachineOutput } from './machineOutput';
import {
  ids,
  inputDatabaseRunIds,
  querySourceOptions,
  querySourceUsage,
  withQuerySources,
  withReusableQuerySources,
} from './querySources';

export const jsonSessionUsage = `Usage:
  pnpm ask session [options] --json

Options:
${querySourceUsage}
  --json                  Read JSON Lines requests from stdin

Request:
  {"command":"queries","args":["--source","RUN_ID","--engine","ENGINE_ID"]}

Send {"command":"exit"} or close stdin to stop the session.`;

interface JsonSessionRequest {
  command: string;
  args: string[];
}

function parseRequest(line: string): JsonSessionRequest {
  const value: unknown = JSON.parse(line);
  if (!value || typeof value !== 'object') {
    throw new Error('Session request must be a JSON object.');
  }
  const command = Reflect.get(value, 'command');
  const args = Reflect.get(value, 'args') ?? [];
  if (typeof command !== 'string' || command.trim() === '') {
    throw new Error('Session request requires a non-empty string "command".');
  }
  if (!Array.isArray(args) || args.some(arg => typeof arg !== 'string')) {
    throw new Error('Session request "args" must be an array of strings.');
  }
  return { command, args };
}

export async function runJsonSessionRequests(
  lines: AsyncIterable<string>,
  write: (output: string) => void = output => process.stdout.write(output)
): Promise<void> {
  await withCompactMachineOutput(async () => {
    for await (const line of lines) {
      if (!line.trim()) {
        continue;
      }
      let commandId: string | null = null;
      try {
        const request = parseRequest(line);
        commandId = request.command;
        if (commandId === 'exit') {
          return;
        }
        const command = getAskCommand(commandId);
        const args = request.args.includes('--json') ? request.args : [...request.args, '--json'];
        await command.run(args);
      } catch (error) {
        write(serializeMachineOutput(machineError(commandId, error)));
      }
    }
  });
}

export async function runJsonSession(
  args: string[],
  input: Readable = process.stdin
): Promise<void> {
  const { values } = parseArgs({
    args,
    allowPositionals: false,
    options: {
      ...querySourceOptions,
      json: { type: 'boolean' },
    },
  });
  if (!values.json) {
    throw new Error('Session mode requires --json.');
  }
  if (values.db && ids(values['db-run']).length === 0) {
    throw new Error('JSON session mode requires explicit --db-run values instead of --db.');
  }

  await withReusableQuerySources(async () => {
    await withQuerySources(values, inputDatabaseRunIds, async () => {
      const lines = createInterface({ input, crlfDelay: Infinity });
      await runJsonSessionRequests(lines);
    });
  });
}
