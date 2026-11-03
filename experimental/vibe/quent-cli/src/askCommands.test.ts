// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { askCommandRegistry, askUsage, getAskCommand } from './askCommands';

describe('ask command dispatcher', () => {
  it('registers discovery commands', () => {
    expect([...askCommandRegistry.keys()]).toEqual(['engines', 'queries']);
    expect(askUsage).toContain('pnpm ask <command>');
    expect(askUsage).toContain('engines');
    expect(askUsage).toContain('queries');
    expect(askUsage).not.toContain('query-groups');
  });

  it('provides command-specific usage', () => {
    expect(getAskCommand('engines').usage).toContain('pnpm ask engines');
    expect(getAskCommand('queries').usage).toContain('pnpm ask queries');
  });

  it('rejects unknown commands with the supported command list', () => {
    expect(() => getAskCommand('unknown')).toThrow('Supported commands: engines, queries');
  });
});
