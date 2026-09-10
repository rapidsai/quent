// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useMemo } from 'react';

import { useTimelineEchartsTheme } from '../timeline/timelineEchartsTheme';
import {
  useSelectedOperatorIds,
  useOperatorSelection,
  useOperatorSelectionActions,
  useSetSelectedPlanId,
  useNodeColoringValue,
  useNodeColorPalette,
  COLOR_REGISTRY_KEYS,
  useColorResolver,
} from '@quent/hooks';
import { continuousColor, withOpacity, toggleOperatorSelection, type Operator } from '@quent/utils';
import type { OperatorActiveSpanEntry } from './types';
import { GanttChart, type GanttRenderItem } from '../gantt-chart/GanttChart';
import type { GanttHover } from '../gantt-chart/hover';
import { layoutGanttBar } from '../gantt-chart/utils';
import { getOperatorsAtTimestamp } from './utils';
import { GanttTooltipPortal, type GanttTooltipItem } from '../ui/gantt-tooltip';

const DEFAULT_HEIGHT = 75;
const MAX_HEIGHT = 200;
const BAR_FONT_SIZE = 10;
const BAR_HEIGHT = 16;
const BAR_GAP = 2;

function getOperatorBarColors(
  typeName: string | undefined,
  resolveColor: (value: string) => string
): { fill: string; stroke: string } {
  const stroke = resolveColor(typeName ?? '');
  return { stroke, fill: withOpacity(stroke, 0.45) };
}

export interface OperatorGanttChartProps {
  operators: OperatorActiveSpanEntry[];
  allOperators: readonly Operator[];
  durationSeconds: number;
  height?: number;
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
}

export function OperatorGanttChart({
  operators,
  allOperators,
  durationSeconds,
  height = DEFAULT_HEIGHT,
  isDark,
}: OperatorGanttChartProps) {
  const operatorSelection = useOperatorSelection();
  const updateOperatorSelection = useOperatorSelectionActions();
  const setSelectedPlanId = useSetSelectedPlanId();
  const { textColor } = useTimelineEchartsTheme(isDark);
  const nodeColoring = useNodeColoringValue();
  const [nodePalette] = useNodeColorPalette();
  const resolveOperatorTypeColor = useColorResolver(COLOR_REGISTRY_KEYS.OPERATOR_TYPES);
  const barLabelTextColor = textColor;
  const selectedOperatorIds = useSelectedOperatorIds();
  const chartLabel = `Operator Gantt chart: ${operators.map(operator => operator.label).join(', ')}`;
  const selectedVisibleOperatorIds = operators
    .filter(operator => selectedOperatorIds.has(operator.operatorId))
    .map(operator => operator.operatorId)
    .sort();

  const customSeriesData = useMemo(
    () =>
      operators.map(op => ({
        value: [op.startMs, op.endMs, op.rowIndex] as [number, number, number],
        name: op.label,
      })),
    [operators]
  );
  const renderTooltip = useCallback(
    (hover: GanttHover | null) => {
      const items: GanttTooltipItem[] = hover
        ? getOperatorsAtTimestamp(operators, hover.timestampMs).map(operator => ({
            id: operator.operatorId,
            color: getOperatorBarColors(operator.typeName, resolveOperatorTypeColor).stroke,
            name: operator.label,
          }))
        : [];
      return <GanttTooltipPortal hover={hover} items={items} />;
    },
    [operators, resolveOperatorTypeColor]
  );
  const operatorFieldStyles = useMemo(() => {
    const styles = new Map<string, { stroke?: string; fieldDimmed: boolean }>();
    if (!nodeColoring) {
      return styles;
    }
    for (const op of operators) {
      if (styles.has(op.operatorId)) {
        continue;
      }
      if (nodeColoring.type === 'continuous') {
        const v = nodeColoring.values.get(op.operatorId);
        if (v === undefined) {
          styles.set(op.operatorId, { stroke: undefined, fieldDimmed: true });
          continue;
        }
        const t =
          nodeColoring.max > nodeColoring.min
            ? (v - nodeColoring.min) / (nodeColoring.max - nodeColoring.min)
            : 0.5;
        styles.set(op.operatorId, {
          stroke: continuousColor(t, nodePalette, isDark),
          fieldDimmed: false,
        });
      } else {
        const stroke = nodeColoring.colorMap.get(op.operatorId);
        styles.set(op.operatorId, { stroke, fieldDimmed: !stroke });
      }
    }
    return styles;
  }, [operators, nodeColoring, nodePalette, isDark]);
  const renderItem: GanttRenderItem = useCallback(
    (params, api) => {
      const layout = layoutGanttBar(params, api, {
        barHeight: Math.max(1, BAR_HEIGHT - BAR_GAP),
      });
      if (!layout) {
        return null;
      }
      const { clippedShape } = layout;

      const op = operators[params.dataIndexInside];
      const barLabel =
        op?.typeName && op.typeName !== op.label
          ? `${op.typeName}: ${op.label}`
          : (op?.label ?? '');
      const { fill } = getOperatorBarColors(op?.typeName, resolveOperatorTypeColor);
      const fieldStyle = op ? operatorFieldStyles.get(op.operatorId) : undefined;
      const hasSelection = selectedOperatorIds.size > 0;
      const isSelected = op != null && selectedOperatorIds.has(op.operatorId);
      const fieldDimmed = fieldStyle?.fieldDimmed ?? false;
      const opacity = fieldDimmed || (hasSelection && !isSelected) ? 0.35 : 1;

      const rect = {
        type: 'rect' as const,
        shape: { ...clippedShape, r: 2 },
        style: {
          fill,
          lineWidth: 1,
          opacity,
        },
      };

      const text = {
        type: 'text' as const,
        style: {
          text: barLabel,
          x: clippedShape.x + 6,
          y: clippedShape.y + clippedShape.height / 2,
          textVerticalAlign: 'middle' as const,
          fontSize: BAR_FONT_SIZE,
          fill: barLabelTextColor,
          overflow: 'truncate' as const,
          width: Math.max(0, clippedShape.width - 12),
          opacity,
        },
      };

      return {
        type: 'group' as const,
        children: [rect, text],
      };
    },
    [
      operators,
      operatorFieldStyles,
      barLabelTextColor,
      selectedOperatorIds,
      resolveOperatorTypeColor,
    ]
  );

  const handleClick = useMemo(
    () => ({
      click: (params: { dataIndex: number; seriesName?: string }) => {
        if (params.seriesName !== 'operator-span') {
          return;
        }
        const op = operators[params.dataIndex];
        if (!op) {
          return;
        }
        if (selectedOperatorIds.has(op.operatorId)) {
          updateOperatorSelection({
            type: 'replace',
            selections: toggleOperatorSelection(
              allOperators,
              selectedOperatorIds,
              operatorSelection.selections,
              op.operatorId
            ),
          });
        } else {
          updateOperatorSelection({
            type: 'add',
            selectionId: op.operatorId,
            label: op.label,
            operatorIds: [op.operatorId],
            selectedData: {
              nodeId: op.operatorId,
              label: op.label,
              operationType: op.typeName,
              statistics: op.statistics,
            },
          });
          if (op.planId) {
            setSelectedPlanId(op.planId);
          }
        }
      },
    }),
    [
      allOperators,
      operators,
      operatorSelection.selections,
      selectedOperatorIds,
      setSelectedPlanId,
      updateOperatorSelection,
    ]
  );

  return (
    <div
      role="group"
      aria-label={chartLabel}
      data-selected-operator-ids={selectedVisibleOperatorIds.join(' ')}
    >
      <GanttChart
        data={customSeriesData}
        durationSeconds={durationSeconds}
        height={height}
        maxHeight={MAX_HEIGHT}
        rowHeight={BAR_HEIGHT}
        isDark={isDark}
        seriesName="operator-span"
        renderItem={renderItem}
        emptyMessage="No operator active spans"
        cursor="pointer"
        onEvents={handleClick}
        renderTooltip={renderTooltip}
      />
    </div>
  );
}
