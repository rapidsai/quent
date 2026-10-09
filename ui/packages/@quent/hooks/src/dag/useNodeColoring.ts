// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import { useSelectedOperatorIds } from './useSelectedOperatorIds';
import { useAtomValue } from 'jotai';
import {
  nodeColoringAtom,
  selectedColorField,
  nodeColorPaletteAtom,
  selectedScaleTypeAtom,
} from '../atoms/dagControls';
import { continuousColor, normalizeScaleValue } from '@quent/utils';

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

export function isOperatorGroupSelected(
  selectedOperatorIds: ReadonlySet<string>,
  operatorId: string,
  relatedOperatorIds: readonly string[] = []
): boolean {
  return (
    selectedOperatorIds.has(operatorId) &&
    relatedOperatorIds.every(id => selectedOperatorIds.has(id))
  );
}

/**
 * Returns coloring state for a DAG node.
 * @param operatorId - The node's operator ID
 * @param isDark - Whether the UI is in dark mode (replaces useTheme dependency)
 */
export function useNodeColoring(
  operatorId: string,
  isDark: boolean,
  relatedOperatorIds: readonly string[] = []
): NodeColoringResult {
  const selectedOperatorIds = useSelectedOperatorIds();
  const nodeColoring = useAtomValue(nodeColoringAtom);
  const nodePalette = useAtomValue(nodeColorPaletteAtom);
  const colorField = useAtomValue(selectedColorField);
  const scaleType = useAtomValue(selectedScaleTypeAtom);

  const isSelected = isOperatorGroupSelected(selectedOperatorIds, operatorId, relatedOperatorIds);

  const { fieldColor, fieldDimmed } = useMemo(() => {
    if (!nodeColoring) {
      return { fieldColor: undefined, fieldDimmed: false };
    }
    if (nodeColoring.type === 'continuous') {
      const v = nodeColoring.values.get(operatorId);
      if (v === undefined) {
        return { fieldColor: undefined, fieldDimmed: true };
      }
      const t = normalizeScaleValue(v, nodeColoring, scaleType);
      return { fieldColor: continuousColor(t, nodePalette, isDark), fieldDimmed: false };
    }
    const color = nodeColoring.colorMap.get(operatorId);
    return { fieldColor: color, fieldDimmed: !color };
  }, [nodeColoring, operatorId, nodePalette, isDark, scaleType]);

  const hasSelection = selectedOperatorIds.size > 0;
  const isDimmed = fieldDimmed || (hasSelection && !isSelected);

  return { fieldColor, fieldDimmed, isDimmed, isSelected, colorField };
}
