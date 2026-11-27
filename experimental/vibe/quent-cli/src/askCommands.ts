// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { questionRegistry } from './questions/questionRegistry';
import type { AskCommand } from './askCommand.types';
import { discoveryCommands } from './discoveryCommands';
import { createQuestionCommand } from './questionCommand';
import { queryDiffCommand } from './queryDiff';

const commands = [
  ...discoveryCommands,
  ...[...questionRegistry.values()].map(createQuestionCommand),
  queryDiffCommand,
];

export const askCommandRegistry: ReadonlyMap<string, AskCommand> = new Map(
  commands.map(command => [command.id, command])
);

export const askUsage = `Usage:
  pnpm ask <command> [options]

Commands:
${commands.map(command => `  ${command.id.padEnd(24)} ${command.explanation}`).join('\n')}

Run "pnpm ask <command> --help" for command options.`;

export function getAskCommand(commandId: string): AskCommand {
  const command = askCommandRegistry.get(commandId);
  if (!command) {
    throw new Error(
      `Unknown ask command "${commandId}". Supported commands: ${[...askCommandRegistry.keys()].join(', ')}.`
    );
  }
  return command;
}
