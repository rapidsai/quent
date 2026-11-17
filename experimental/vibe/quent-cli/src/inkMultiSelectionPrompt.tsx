// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import * as React from 'react';
import { Box, Text, useApp, useInput } from 'ink';
import type { SelectionChoice } from './askSelection';
import {
  filterSelectionChoices,
  MAX_VISIBLE_CHOICES,
  selectionWindow,
  toggleMultiSelection,
} from './inkSelector.utils';

interface MultiSelectionPromptProps {
  prompt: string;
  choices: readonly SelectionChoice[];
  allValue?: string;
}

export function MultiSelectionPrompt({ prompt, choices, allValue }: MultiSelectionPromptProps) {
  const { exit } = useApp();
  const [filter, setFilter] = React.useState('');
  const [selectedIndex, setSelectedIndex] = React.useState(0);
  const [selectedValues, setSelectedValues] = React.useState<ReadonlySet<string>>(new Set());
  const filtered = React.useMemo(() => filterSelectionChoices(choices, filter), [choices, filter]);
  const boundedIndex = Math.min(selectedIndex, Math.max(0, filtered.length - 1));
  const visible = selectionWindow(filtered, boundedIndex);

  useInput((input, key) => {
    if (key.ctrl && input === 'c') {
      exit(new Error('Selection cancelled.'));
      return;
    }
    if (key.escape) {
      if (filter) {
        setFilter('');
        setSelectedIndex(0);
      } else {
        exit(new Error('Selection cancelled.'));
      }
      return;
    }
    if (key.upArrow || (key.tab && key.shift)) {
      setSelectedIndex(index => Math.max(0, Math.min(index, filtered.length - 1) - 1));
      return;
    }
    if (key.downArrow || key.tab) {
      setSelectedIndex(index => Math.min(filtered.length - 1, Math.max(0, index) + 1));
      return;
    }
    if (key.pageUp) {
      setSelectedIndex(index => Math.max(0, index - MAX_VISIBLE_CHOICES));
      return;
    }
    if (key.pageDown) {
      setSelectedIndex(index => Math.min(filtered.length - 1, index + MAX_VISIBLE_CHOICES));
      return;
    }
    if (key.home) {
      setSelectedIndex(0);
      return;
    }
    if (key.end) {
      setSelectedIndex(Math.max(0, filtered.length - 1));
      return;
    }
    if (key.ctrl && input === 'a') {
      setSelectedValues(
        allValue ? new Set([allValue]) : new Set(filtered.map(choice => choice.value))
      );
      return;
    }
    if (input === ' ') {
      const highlighted = filtered[boundedIndex];
      if (highlighted) {
        setSelectedValues(current => toggleMultiSelection(current, highlighted.value, allValue));
      }
      return;
    }
    if (key.return) {
      const selected = choices
        .filter(choice => selectedValues.has(choice.value))
        .map(choice => choice.value);
      if (selected.length > 0) {
        exit(selected);
      }
      return;
    }
    if (key.backspace || key.delete) {
      setFilter(value => value.slice(0, -1));
      setSelectedIndex(0);
      return;
    }
    if (input && !key.ctrl && !key.meta) {
      const printable = [...input].filter(character => character >= ' ').join('');
      if (printable) {
        setFilter(value => value + printable);
        setSelectedIndex(0);
      }
    }
  });

  return (
    <Box flexDirection="column">
      <Box borderStyle="round" borderColor="cyan" flexDirection="column" paddingX={1}>
        <Box justifyContent="space-between">
          <Text bold color="cyan">
            QUENT
          </Text>
          <Text color={selectedValues.size > 0 ? 'yellow' : undefined}>
            {selectedValues.size} selected
          </Text>
        </Box>
        <Text bold>{prompt}</Text>
        <Box>
          <Text dimColor>Filter: </Text>
          <Text color={filter ? 'yellow' : undefined}>{filter || 'type to search'}</Text>
        </Box>
        <Box flexDirection="column" marginTop={1}>
          {visible.choices.map((choice, visibleIndex) => {
            const index = visible.offset + visibleIndex;
            const highlighted = index === boundedIndex;
            const selected = selectedValues.has(choice.value);
            return (
              <Text
                key={choice.value}
                color={highlighted ? 'yellow' : selected ? 'cyan' : undefined}
              >
                {highlighted ? '› ' : '  '}
                {selected ? '● ' : '○ '}
                {choice.label}
              </Text>
            );
          })}
          {filtered.length === 0 && <Text color="red"> No matching metrics</Text>}
        </Box>
      </Box>
      <Text dimColor>
        ↑/↓ navigate · space toggle · enter confirm · ctrl+a select all · esc{' '}
        {filter ? 'clear' : 'cancel'}
      </Text>
    </Box>
  );
}
