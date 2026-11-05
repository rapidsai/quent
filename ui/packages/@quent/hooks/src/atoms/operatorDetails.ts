// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { SelectedOperatorGroupData } from '@quent/utils';
import { operatorSelectionAtom } from './dag';
import { selectedOperatorsDataAtom } from './dagControls';

export interface OperatorDetailEntry {
  selectionId: string;
  data: SelectedOperatorGroupData;
}

export const displayedOperatorDetailsAtom = atom<OperatorDetailEntry[]>(get => {
  const selectedData = get(selectedOperatorsDataAtom);
  return [...get(operatorSelectionAtom).selections].flatMap(([selectionId]) => {
    const data = selectedData.get(selectionId);
    return data ? [{ selectionId, data }] : [];
  });
});
