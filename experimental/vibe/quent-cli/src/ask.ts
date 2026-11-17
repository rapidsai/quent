// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { askUsage, getAskCommand } from './askCommands';
import { machineError, serializeMachineOutput } from './machineOutput';

function isParseArgsError(error: unknown): boolean {
  return (
    error instanceof Error &&
    'code' in error &&
    typeof error.code === 'string' &&
    error.code.startsWith('ERR_PARSE_ARGS_')
  );
}

const [commandId, ...args] = process.argv.slice(2);
let selectedUsage = askUsage;
try {
  if (!commandId || commandId === '--help' || commandId === '-h') {
    process.stdout.write(`${askUsage}\n`);
  } else {
    const command = getAskCommand(commandId);
    selectedUsage = command.usage;
    if (args.length === 1 && (args[0] === '--help' || args[0] === '-h')) {
      process.stdout.write(`${command.usage}\n`);
    } else {
      await command.run(args);
    }
  }
} catch (error) {
  if (args.includes('--json')) {
    process.stderr.write(serializeMachineOutput(machineError(commandId ?? null, error)));
  } else {
    const message = error instanceof Error ? error.message : String(error);
    const showUsage = selectedUsage === askUsage || isParseArgsError(error);
    process.stderr.write(`${message}${showUsage ? `\n\n${selectedUsage}` : ''}\n`);
  }
  process.exitCode = 1;
}
