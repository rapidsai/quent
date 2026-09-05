// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValue, useSetAtom } from 'jotai';
import { operatorSelectionActionAtom, operatorSelectionAtom } from '../atoms/dag';

export type { OperatorSelectionAction } from '../atoms/dag';

export const useOperatorSelection = () => useAtomValue(operatorSelectionAtom);
export const useOperatorSelectionActions = () => useSetAtom(operatorSelectionActionAtom);
