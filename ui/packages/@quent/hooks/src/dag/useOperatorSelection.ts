// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValueRawSync, useSetAtom } from 'jotai';
import { operatorSelectionActionAtom, operatorSelectionAtom } from '../atoms/dag';

export const useOperatorSelection = () => useAtomValueRawSync(operatorSelectionAtom);
export const useOperatorSelectionActions = () => useSetAtom(operatorSelectionActionAtom);
