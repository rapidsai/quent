// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValue, useSetAtom } from 'jotai';
import { hoveredWorkerIdAtom } from '../atoms/dag';

export const useHoveredWorkerId = () => useAtomValue(hoveredWorkerIdAtom);
export const useSetHoveredWorkerId = () => useSetAtom(hoveredWorkerIdAtom);
