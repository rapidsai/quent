// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValue, useSetAtom } from 'jotai';
import { selectedPlanIdAtom } from '../atoms/dag';

export const useSelectedPlanId = () => useAtomValue(selectedPlanIdAtom);
export const useSetSelectedPlanId = () => useSetAtom(selectedPlanIdAtom);
