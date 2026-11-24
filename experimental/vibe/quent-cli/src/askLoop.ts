// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { AskCommand } from './askCommand.types';
import { askCommandRegistry, getAskCommand } from './askCommands';
import type { SelectFromList } from './askSelection';

const EXIT_COMMAND = '__exit';

export function shouldRunCommandLoop(
  args: readonly string[],
  inputIsTTY = process.stdin.isTTY,
  outputIsTTY = process.stderr.isTTY
): boolean {
  const usesDatabase = args.some(
    arg => arg === '--db' || arg === '--db-run' || arg.startsWith('--db-run=')
  );
  return Boolean(inputIsTTY && outputIsTTY && usesDatabase && !args.includes('--json'));
}

export async function runAskCommandLoop(
  initialCommandId: string,
  initialArgs: string[],
  select: SelectFromList,
  onCommand: (command: AskCommand) => void = () => undefined
): Promise<void> {
  let commandId = initialCommandId;
  let args = initialArgs;
  while (true) {
    const command = getAskCommand(commandId);
    onCommand(command);
    await command.run(args);

    commandId = await select('Run another command', [
      ...[...askCommandRegistry.values()].map(candidate => ({
        value: candidate.id,
        label: `${candidate.id} — ${candidate.explanation}`,
      })),
      { value: EXIT_COMMAND, label: 'Exit' },
    ]);
    if (commandId === EXIT_COMMAND) {
      return;
    }
    args = [];
  }
}
