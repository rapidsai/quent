// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Selector hooks for data-flow atoms (HOOKS-02: no raw atom exports).

import { useAtomValue, useSetAtom } from 'jotai';
import {
  dataFlowEnabledAtom,
  playheadTimeSAtom,
  selectedDataFlowMeasureAtom,
  dataFlowMetaAtom,
  dataFlowFrameAtom,
} from '../atoms/dataFlow';

export const useDataFlowEnabled = () => useAtomValue(dataFlowEnabledAtom);
export const useSetDataFlowEnabled = () => useSetAtom(dataFlowEnabledAtom);

export const usePlayheadTimeS = () => useAtomValue(playheadTimeSAtom);
export const useSetPlayheadTimeS = () => useSetAtom(playheadTimeSAtom);

export const useSelectedDataFlowMeasure = () => useAtomValue(selectedDataFlowMeasureAtom);
export const useSetSelectedDataFlowMeasure = () => useSetAtom(selectedDataFlowMeasureAtom);

export const useDataFlowMeta = () => useAtomValue(dataFlowMetaAtom);
export const useDataFlowFrame = () => useAtomValue(dataFlowFrameAtom);
