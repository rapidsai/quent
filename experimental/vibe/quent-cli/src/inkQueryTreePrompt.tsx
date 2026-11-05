// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import * as React from 'react';
import { Box, Text, useApp, useInput } from 'ink';
import type { QueryTreeChoice } from './askSelection';
import { filterQueryTreeChoices, selectionWindow } from './inkSelector.utils';

interface QueryTreePromptProps {
  prompt: string;
  choices: readonly QueryTreeChoice[];
  multiple: boolean;
}

export function QueryTreePrompt({ prompt, choices, multiple }: QueryTreePromptProps) {
  const { exit } = useApp();
  const [filter, setFilter] = React.useState('');
  const [selectedIndex, setSelectedIndex] = React.useState(0);
  const [selectedValues, setSelectedValues] = React.useState<ReadonlySet<string>>(new Set());
  const filtered = React.useMemo(() => filterQueryTreeChoices(choices, filter), [choices, filter]);
  const boundedIndex = Math.min(selectedIndex, Math.max(0, filtered.length - 1));
  const visible = selectionWindow(filtered, boundedIndex, 8);

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
      setSelectedIndex(index => Math.max(0, index - 8));
      return;
    }
    if (key.pageDown) {
      setSelectedIndex(index => Math.min(filtered.length - 1, index + 8));
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
    if (multiple && key.ctrl && input === 'a') {
      setSelectedValues(new Set(filtered.map(choice => choice.value)));
      return;
    }
    if (multiple && input === ' ') {
      const highlighted = filtered[boundedIndex];
      if (highlighted) {
        setSelectedValues(current => {
          const next = new Set(current);
          if (next.has(highlighted.value)) {
            next.delete(highlighted.value);
          } else {
            next.add(highlighted.value);
          }
          return next;
        });
      }
      return;
    }
    if (key.return) {
      if (multiple) {
        const selected = choices
          .filter(choice => selectedValues.has(choice.value))
          .map(choice => choice.value);
        if (selected.length > 0) {
          exit(selected);
        }
      } else {
        const highlighted = filtered[boundedIndex];
        if (highlighted) {
          exit([highlighted.value]);
        }
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
            {multiple ? `${selectedValues.size} selected` : 'select one'}
          </Text>
        </Box>
        <Text bold>{prompt}</Text>
        <Box>
          <Text dimColor>Filter all engines and queries: </Text>
          <Text color={filter ? 'yellow' : undefined}>{filter || 'type to search'}</Text>
        </Box>
        <Box flexDirection="column" marginTop={1}>
          {visible.choices.map((choice, visibleIndex) => {
            const previous = visible.choices[visibleIndex - 1];
            const showEngine =
              !previous ||
              previous.sourceId !== choice.sourceId ||
              previous.engineId !== choice.engineId;
            const index = visible.offset + visibleIndex;
            const highlighted = index === boundedIndex;
            const selected = selectedValues.has(choice.value);
            return (
              <React.Fragment key={choice.value}>
                {showEngine && (
                  <Text bold color="blue">
                    ◆ {choice.engineLabel}
                  </Text>
                )}
                <Text color={highlighted ? 'yellow' : selected ? 'cyan' : undefined}>
                  {highlighted ? '  › ' : '    '}
                  {multiple ? (selected ? '● ' : '○ ') : ''}
                  {choice.label}
                </Text>
              </React.Fragment>
            );
          })}
          {filtered.length === 0 && <Text color="red"> No matching queries</Text>}
        </Box>
        {filtered.length > 8 && (
          <Text dimColor>
            Showing queries {visible.offset + 1}–{Math.min(visible.offset + 8, filtered.length)} of{' '}
            {filtered.length}
          </Text>
        )}
      </Box>
      <Text dimColor>
        ↑/↓ navigate · type to filter · {multiple ? 'space toggle · ' : ''}enter{' '}
        {multiple ? 'confirm · ctrl+a select filtered' : 'select'} · esc{' '}
        {filter ? 'clear' : 'cancel'}
      </Text>
    </Box>
  );
}
