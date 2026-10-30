// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValue, useSetAtom } from 'jotai';
import {
  displayedOperatorDetailsAtom,
  operatorDetailsActionAtom,
  pinnedOperatorDetailIdsAtom,
} from '../atoms/operatorDetails';

export const useDisplayedOperatorDetails = () => useAtomValue(displayedOperatorDetailsAtom);
export const usePinnedOperatorDetailIds = () => useAtomValue(pinnedOperatorDetailIdsAtom);
export const useOperatorDetailsActions = () => useSetAtom(operatorDetailsActionAtom);
