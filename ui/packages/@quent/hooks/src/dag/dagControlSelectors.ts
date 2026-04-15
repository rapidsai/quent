// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Selector hooks for DAG control atoms (HOOKS-02: no raw atom exports).
// Components use these hooks to read/write DAG visual control state.

import { useAtomValue, useSetAtom, useAtom } from 'jotai';
import {
  selectedColorField,
  nodeColoringAtom,
  nodeColorPaletteAtom,
  selectedEdgeWidthFieldAtom,
  edgeWidthConfigAtom,
  selectedEdgeColorFieldAtom,
  edgeColoringAtom,
  edgeColorPaletteAtom,
  selectedNodeLabelFieldAtom,
} from '../atoms/dagControls';

export function useSelectedColorField() {
  return useAtom(selectedColorField);
}

export function useNodeColoringValue() {
  return useAtomValue(nodeColoringAtom);
}

export function useSetNodeColoring() {
  return useSetAtom(nodeColoringAtom);
}

export function useNodeColorPalette() {
  return useAtom(nodeColorPaletteAtom);
}

export function useSelectedEdgeWidthField() {
  return useAtom(selectedEdgeWidthFieldAtom);
}

export function useEdgeWidthConfig() {
  return useAtomValue(edgeWidthConfigAtom);
}

export function useSelectedEdgeColorField() {
  return useAtom(selectedEdgeColorFieldAtom);
}

export function useEdgeColoring() {
  return useAtomValue(edgeColoringAtom);
}

export function useEdgeColorPalette() {
  return useAtom(edgeColorPaletteAtom);
}

export function useSelectedNodeLabelField() {
  return useAtom(selectedNodeLabelFieldAtom);
}
