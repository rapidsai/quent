// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { QueryTreeChoice, SelectionChoice } from './askSelection';

export const MAX_VISIBLE_CHOICES = 10;

export function filterSelectionChoices(
  choices: readonly SelectionChoice[],
  filter: string
): readonly SelectionChoice[] {
  const normalized = filter.trim().toLocaleLowerCase();
  if (!normalized) {
    return choices;
  }
  return choices.filter(choice =>
    `${choice.label} ${choice.value}`.toLocaleLowerCase().includes(normalized)
  );
}

export function filterQueryTreeChoices(
  choices: readonly QueryTreeChoice[],
  filter: string
): readonly QueryTreeChoice[] {
  const normalized = filter.trim().toLocaleLowerCase();
  if (!normalized) {
    return choices;
  }
  return choices.filter(choice =>
    [choice.sourceId, choice.engineLabel, choice.engineId, choice.label, choice.queryId]
      .join(' ')
      .toLocaleLowerCase()
      .includes(normalized)
  );
}

export function selectionWindow<T extends SelectionChoice>(
  choices: readonly T[],
  selectedIndex: number,
  maximum = MAX_VISIBLE_CHOICES
): { choices: readonly T[]; offset: number } {
  if (choices.length <= maximum) {
    return { choices, offset: 0 };
  }
  const centered = selectedIndex - Math.floor(maximum / 2);
  const offset = Math.max(0, Math.min(centered, choices.length - maximum));
  return { choices: choices.slice(offset, offset + maximum), offset };
}

export function toggleMultiSelection(
  current: ReadonlySet<string>,
  value: string,
  allValue?: string
): ReadonlySet<string> {
  if (value === allValue) {
    return current.has(value) ? new Set() : new Set([value]);
  }
  const next = new Set(current);
  if (allValue) {
    next.delete(allValue);
  }
  if (next.has(value)) {
    next.delete(value);
  } else {
    next.add(value);
  }
  return next;
}
