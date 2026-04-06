// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { useAtomValue } from 'jotai';
import {
  selectedNodeIdsAtom,
  nodeColoringAtom,
  selectedColorField,
  nodeColorPaletteAtom,
} from '@/atoms/dag';
import { continuousColor } from '@/services/colors';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';

interface NodeColoringResult {
  /** Computed hex color for this node, or undefined if no coloring is active */
  fieldColor: string | undefined;
  /** True when the node has no value for the active color field */
  fieldDimmed: boolean;
  /** True when this node should be visually de-emphasized (no data or selection excludes it) */
  isDimmed: boolean;
  /** True when this node is in the active selection set */
  isSelected: boolean;
  /** The currently active color field name, or null */
  colorField: string | null;
}

export function useNodeColoring(operatorId: string): NodeColoringResult {
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const nodeColoring = useAtomValue(nodeColoringAtom);
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const colorField = useAtomValue(selectedColorField);

  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;
  const isSelected = selectedNodeIds.has(operatorId);

  const { fieldColor, fieldDimmed } = useMemo(() => {
    if (!nodeColoring) return { fieldColor: undefined, fieldDimmed: false };
    if (nodeColoring.type === 'continuous') {
      const v = nodeColoring.values.get(operatorId);
      if (v === undefined) return { fieldColor: undefined, fieldDimmed: true };
      const t =
        nodeColoring.max > nodeColoring.min
          ? (v - nodeColoring.min) / (nodeColoring.max - nodeColoring.min)
          : 0.5;
      return { fieldColor: continuousColor(t, nodePalette, isDarkMode), fieldDimmed: false };
    }
    const color = nodeColoring.colorMap.get(operatorId);
    return { fieldColor: color, fieldDimmed: !color };
  }, [nodeColoring, operatorId, nodePalette, isDarkMode]);

  const hasSelection = selectedNodeIds.size > 0;
  const isDimmed = fieldDimmed || (hasSelection && !isSelected);

  return { fieldColor, fieldDimmed, isDimmed, isSelected, colorField };
}
