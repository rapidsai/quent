// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parse } from 'json-custom-numbers';

/**
 * Parse JSON with BigInt support for large integer number tokens.
 *
 * Integers outside the safe JavaScript number range are converted to BigInt.
 * Strings are parsed as strings, even when their contents look like large
 * integers.
 */
export function parseJsonWithBigInt<T>(text: string): T {
  return parse(text, undefined, parseUnsafeInteger) as T;
}

function parseUnsafeInteger(_key: string | number | undefined, source: string): number | bigint {
  const value = Number(source);

  if (isIntegerLiteral(source) && !Number.isSafeInteger(value)) {
    return BigInt(source);
  }

  return value;
}

function isIntegerLiteral(source: string): boolean {
  return !source.includes('.') && !source.includes('e') && !source.includes('E');
}
