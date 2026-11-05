// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { getQuestion, questionRegistry } from './questionRegistry';

describe('question registry', () => {
  it('registers stable question metadata and rejects unknown questions', () => {
    expect([...questionRegistry.keys()]).toEqual([
      'query-lifecycle',
      'query-structure',
      'operator-facts',
      'operator-statistics-summary',
    ]);
    for (const id of questionRegistry.keys()) {
      expect(getQuestion(id).metadata).toMatchObject({ id, version: 1 });
    }
    expect(() => getQuestion('resource-magic')).toThrow(
      'Supported questions: query-lifecycle, query-structure, operator-facts, operator-statistics-summary'
    );
  });
});
