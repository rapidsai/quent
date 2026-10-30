// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import * as React from 'react';
import { Box, Text, useApp, useInput } from 'ink';

interface TextPromptProps {
  prompt: string;
  placeholder?: string;
}

export function TextPrompt({ prompt, placeholder }: TextPromptProps) {
  const { exit } = useApp();
  const [value, setValue] = React.useState('');

  useInput((input, key) => {
    if ((key.ctrl && input === 'c') || key.escape) {
      exit(new Error('Selection cancelled.'));
      return;
    }
    if (key.return) {
      if (value.trim()) {
        exit(value.trim());
      }
      return;
    }
    if (key.backspace || key.delete) {
      setValue(current => current.slice(0, -1));
      return;
    }
    if (input && !key.ctrl && !key.meta) {
      const printable = [...input].filter(character => character >= ' ').join('');
      if (printable) {
        setValue(current => current + printable);
      }
    }
  });

  return (
    <Box flexDirection="column">
      <Box borderStyle="round" borderColor="cyan" flexDirection="column" paddingX={1}>
        <Text bold color="cyan">
          QUENT
        </Text>
        <Text bold>{prompt}</Text>
        <Text color={value ? 'yellow' : undefined}>{value || placeholder || 'type a value'}</Text>
      </Box>
      <Text dimColor>type a value · enter confirm · esc cancel</Text>
    </Box>
  );
}
