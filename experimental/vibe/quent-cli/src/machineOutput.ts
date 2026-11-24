// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export const ASK_OUTPUT_SCHEMA_VERSION = 2;

let compactOutput = false;

export interface AskMachineResult<T> {
  schemaVersion: typeof ASK_OUTPUT_SCHEMA_VERSION;
  command: string;
  data: T;
}

export interface AskMachineError {
  schemaVersion: typeof ASK_OUTPUT_SCHEMA_VERSION;
  command: string | null;
  error: {
    message: string;
  };
}

export function machineResult<T>(command: string, data: T): AskMachineResult<T> {
  return {
    schemaVersion: ASK_OUTPUT_SCHEMA_VERSION,
    command,
    data,
  };
}

export function machineError(command: string | null, error: unknown): AskMachineError {
  return {
    schemaVersion: ASK_OUTPUT_SCHEMA_VERSION,
    command,
    error: {
      message: error instanceof Error ? error.message : String(error),
    },
  };
}

export async function withCompactMachineOutput<T>(run: () => Promise<T>): Promise<T> {
  const previous = compactOutput;
  compactOutput = true;
  try {
    return await run();
  } finally {
    compactOutput = previous;
  }
}

export function serializeMachineOutput(value: unknown): string {
  return `${JSON.stringify(
    value,
    (_key, item) => (typeof item === 'bigint' ? item.toString() : item),
    compactOutput ? undefined : 2
  )}\n`;
}
