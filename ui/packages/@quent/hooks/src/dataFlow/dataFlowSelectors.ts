// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Selector hooks for data-flow atoms (HOOKS-02: no raw atom exports).

import { useAtomValue, useSetAtom } from 'jotai';
import {
  dataFlowEnabledAtom,
  playheadTimeSAtom,
  playheadLineTimeMsAtom,
  selectedDataFlowMeasureAtom,
  dataFlowLabelMeasureAtom,
  dataFlowSelectedDimensionsAtom,
  dataFlowMetaAtom,
  dataFlowFrameAtom,
  dataFlowIsPlayingAtom,
} from '../atoms/dataFlow';

export const useDataFlowEnabled = () => useAtomValue(dataFlowEnabledAtom);
export const useSetDataFlowEnabled = () => useSetAtom(dataFlowEnabledAtom);

export const usePlayheadTimeS = () => useAtomValue(playheadTimeSAtom);
export const useSetPlayheadTimeS = () => useSetAtom(playheadTimeSAtom);

export const useSelectedDataFlowMeasure = () => useAtomValue(selectedDataFlowMeasureAtom);
export const useSetSelectedDataFlowMeasure = () => useSetAtom(selectedDataFlowMeasureAtom);

export const useDataFlowLabelMeasure = () => useAtomValue(dataFlowLabelMeasureAtom);
export const useSetDataFlowLabelMeasure = () => useSetAtom(dataFlowLabelMeasureAtom);

export const useDataFlowSelectedDimensions = () => useAtomValue(dataFlowSelectedDimensionsAtom);
export const useSetDataFlowSelectedDimensions = () => useSetAtom(dataFlowSelectedDimensionsAtom);

export const useDataFlowMeta = () => useAtomValue(dataFlowMetaAtom);
export const useDataFlowFrame = () => useAtomValue(dataFlowFrameAtom);

export const useDataFlowIsPlaying = () => useAtomValue(dataFlowIsPlayingAtom);
export const useSetDataFlowIsPlaying = () => useSetAtom(dataFlowIsPlayingAtom);

export const usePlayheadLineTimeMs = () => useAtomValue(playheadLineTimeMsAtom);
export const useSetPlayheadLineTimeMs = () => useSetAtom(playheadLineTimeMsAtom);
