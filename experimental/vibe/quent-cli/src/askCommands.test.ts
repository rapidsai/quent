// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { askCommandRegistry, askUsage, getAskCommand } from './askCommands';

describe('ask command dispatcher', () => {
  it('registers analysis questions and specialized commands', () => {
    expect([...askCommandRegistry.keys()]).toEqual([
      'engines',
      'queries',
      'query-lifecycle',
      'query-structure',
      'operator-facts',
      'operator-statistics-summary',
      'query-diff',
    ]);
    expect(askUsage).toContain('pnpm ask <command>');
    expect(askUsage).not.toContain('query-groups');
    expect(askUsage).toContain('query-lifecycle');
    expect(askUsage).toContain('query-structure');
    expect(askUsage).toContain('operator-facts');
    expect(askUsage).toContain('operator-statistics-summary');
    expect(askUsage).toContain('query-diff');
  });

  it('provides command-specific usage', () => {
    expect(getAskCommand('query-lifecycle').usage).toContain('pnpm ask query-lifecycle');
    expect(getAskCommand('query-structure').usage).toContain('pnpm ask query-structure');
    expect(getAskCommand('operator-facts').usage).toContain('pnpm ask operator-facts');
    expect(getAskCommand('operator-statistics-summary').usage).toContain(
      'pnpm ask operator-statistics-summary'
    );
    expect(getAskCommand('query-diff').usage).toContain('pnpm ask query-diff');
  });

  it('supports database-backed sources in every command', () => {
    for (const command of askCommandRegistry.values()) {
      expect(command.usage).toContain('--db-run');
      expect(command.usage).toContain('--db-api-base-url');
      expect(command.usage).toContain('--db-token');
    }
    for (const id of [
      'engines',
      'queries',
      'query-lifecycle',
      'query-structure',
      'operator-facts',
      'operator-statistics-summary',
    ]) {
      expect(getAskCommand(id).usage).toContain('--source');
    }
  });

  it('allows interactive selection with JSON output', () => {
    for (const command of ['operator-facts', 'query-diff']) {
      const usage = getAskCommand(command).usage;
      expect(usage).toContain('--json');
      expect(usage).toContain('selection may still be interactive');
      expect(usage).not.toContain('never open interactive prompts');
    }
  });

  it('only advertises options used by current questions', () => {
    const usage = getAskCommand('operator-facts').usage;

    expect(usage).toContain('--operator');
    for (const option of ['--resource', '--entity-type', '--start', '--end', '--limit']) {
      expect(usage).not.toContain(option);
    }
  });

  it('rejects unknown commands with the supported command list', () => {
    expect(() => getAskCommand('unknown')).toThrow(
      'Supported commands: engines, queries, query-lifecycle, query-structure, operator-facts, operator-statistics-summary, query-diff'
    );
  });
});
