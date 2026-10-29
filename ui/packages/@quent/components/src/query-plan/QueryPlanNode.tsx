// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { memo, useState, useMemo, useCallback } from 'react';
import { Handle, Position } from '@xyflow/react';
import { cva } from 'class-variance-authority';
import {
  cn,
  scaleColor,
  type ScaleRange,
  formatStatWithQuantity,
  statisticFieldName,
  isLightColor,
  withOpacity,
  WHITE,
  BLACK,
  NODE_LABEL_FIELD,
  DAG_LAYOUT_DIRECTION,
  type Operator,
  type SelectedOperatorGroupData,
  type DagLayoutDirection,
  type QuantitySpec,
  type DAGStatisticSet,
  type Statistic,
} from '@quent/utils';
import {
  useSelectedNodeLabelField,
  useSelectedScaleType,
  useNodeColoring,
  useNodeColorPalette,
  useSetHighlightedNodeIds,
  COLOR_REGISTRY_KEYS,
  useColorResolver,
} from '@quent/hooks';
import { useAtom } from 'jotai';
import { resolveStatisticFields } from '../lib/queryBundle.utils';
import { DataText } from '../ui/data-text';
import { NodeFlowBar } from './NodeFlowBar';
import { getNodeOpacityClass } from './nodeOpacity';
import { DAGItemActions } from '../dag/DAGItemActions';
import { DAGItemStatus } from '../dag/DAGItemStatus';
import { activeDAGPreviewIdAtom } from '../dag/dagPreviewState';
import { OperatorDetailsBlock } from '../node-info/OperatorDetailsBlock';

export interface QueryPlanNodeData extends Record<string, unknown> {
  label: string;
  nodeId: string;
  operationType: string;
  metadata?: {
    rawNode?: Operator;
    relatedOperatorIds?: string[];
    relatedOperators?: Operator[];
    aggregatedStatistics?: Statistic[];
    operatorStatistics?: DAGStatisticSet;
    relatedOperatorStatistics?: DAGStatisticSet[];
    operatorWorkerLabels?: Record<string, string | undefined>;
  };
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
  /** Which edge of the node incoming/outgoing handles attach to; flips with the DAG layout direction. */
  layoutDirection?: DagLayoutDirection;
  /**
   * Whether dark mode is active. Forwarded by `DAGChart` via the node's data
   * payload so the renderer can derive heatmap colors without coupling to a
   * host theme context.
   */
  isDark?: boolean;
  /**
   * Whether the data-flow overlay bar is rendered under the node content.
   * Injected by `DAGChart` when converting nodes so toggling the overlay
   * relayouts exactly once.
   */
  flowBarVisible?: boolean;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
  hoveredStatActive?: boolean;
  hoveredStatValue?: number;
  heatmapRange?: ScaleRange;
  highlightedNodeIds?: ReadonlySet<string> | null;
  actions?: {
    pinned: boolean;
    filtered: boolean;
    details: SelectedOperatorGroupData;
    onTogglePin: () => void;
    onToggleFilter: () => void;
  };
}

const nodeVariants = cva(
  'w-full px-4 py-2 rounded-md border-1 transition cursor-pointer text-foreground z-10 nodrag nopan',
  {
    variants: {
      selected: {
        true: 'border-2 scale-110',
        false: '',
      },
    },
    defaultVariants: {
      selected: false,
    },
  }
);

/** Memoized DAG node rendered inside ReactFlow. */
export const QueryPlanNode = memo(({ data }: { data: QueryPlanNodeData }) => {
  const setHighlightState = useSetHighlightedNodeIds();
  const [nodePalette] = useNodeColorPalette();
  const [scaleType] = useSelectedScaleType();
  const resolveOperatorTypeColor = useColorResolver(COLOR_REGISTRY_KEYS.OPERATOR_TYPES);
  const isDark = data.isDark ?? false;
  const operatorId = data.metadata?.rawNode?.id ?? data.nodeId;
  const highlightedNodeIds = data.highlightedNodeIds ?? null;
  const isHighlighted = highlightedNodeIds?.has(operatorId) === true;
  const { quantitySpecs } = data;
  const [nodeLabelField] = useSelectedNodeLabelField();
  const { fieldColor, isDimmed, isSelected, isFiltered, colorField } = useNodeColoring(
    operatorId,
    isDark,
    data.metadata?.relatedOperatorIds
  );
  const [isHoveredLocal, setIsHoveredLocal] = useState(false);
  const [closedPreviewOperatorIds, setClosedPreviewOperatorIds] = useState<Set<string>>(
    () => new Set()
  );
  const [activePreviewId, setActivePreviewId] = useAtom(activeDAGPreviewIdAtom);
  const previewId = `node:${data.nodeId}`;

  const resolvedLabel = useMemo(() => {
    if (nodeLabelField === NODE_LABEL_FIELD.ID) {
      return data.metadata?.rawNode?.id ?? data.nodeId;
    }
    if (nodeLabelField === NODE_LABEL_FIELD.TYPE) {
      return data.operationType;
    }
    return data.label;
  }, [nodeLabelField, data]);

  const operatorStatistics = data.metadata?.operatorStatistics;
  const relatedOperatorStatistics = data.metadata?.relatedOperatorStatistics;
  const colorFieldStat = colorField
    ? operatorStatistics && relatedOperatorStatistics
      ? resolveStatisticFields(
          operatorStatistics.fields,
          relatedOperatorStatistics.map(statistics => statistics.fields),
          colorField
        )
      : undefined
    : undefined;
  const colorFieldValue = colorFieldStat?.value ?? null;
  const formattedColorFieldValue =
    colorFieldValue === null
      ? null
      : typeof colorFieldValue === 'number' || typeof colorFieldValue === 'bigint'
        ? formatStatWithQuantity(
            colorFieldValue,
            statisticFieldName(colorFieldStat ?? colorField!),
            colorFieldStat?.quantity && quantitySpecs
              ? quantitySpecs[colorFieldStat.quantity]
              : undefined
          )
        : String(colorFieldValue);

  const baseColor = resolveOperatorTypeColor(data.operationType);
  const activeColor = fieldColor ?? baseColor;
  const bgColor =
    fieldColor ?? withOpacity(baseColor, isSelected ? 0.3 : isHoveredLocal ? 0.22 : 0.15);

  const heatmapColor = useMemo(() => {
    if (data.hoveredStatValue === undefined || !data.heatmapRange) {
      return undefined;
    }
    return scaleColor(data.hoveredStatValue, data.heatmapRange, scaleType, nodePalette, isDark);
  }, [data.heatmapRange, data.hoveredStatValue, nodePalette, scaleType, isDark]);

  const opacityClass = getNodeOpacityClass({
    isHoveredStatActive: data.hoveredStatActive ?? false,
    hasHoveredValue: data.hoveredStatValue !== undefined,
    highlightedNodeIds,
    operatorId,
    isDimmed,
    isSelected,
    isFiltered,
  });

  const isActiveHighlight = isHighlighted && !isSelected;

  const isBottomToTop =
    (data.layoutDirection ?? DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP) ===
    DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP;
  const incomingHandlePosition = isBottomToTop ? Position.Bottom : Position.Top;
  const outgoingHandlePosition = isBottomToTop ? Position.Top : Position.Bottom;

  const onMouseEnter = useCallback(() => {
    setIsHoveredLocal(true);
    if (operatorId) {
      setHighlightState(prev => ({
        ...prev,
        ids: new Set([operatorId]),
        source: 'dag',
        primaryOperatorId: operatorId,
      }));
    }
  }, [operatorId, setHighlightState]);
  const onMouseLeave = useCallback(() => {
    setIsHoveredLocal(false);
    setHighlightState(prev =>
      prev.source === 'dag' && prev.ids?.size === 1 && prev.ids.has(operatorId)
        ? { ...prev, ids: null, source: null, primaryOperatorId: null }
        : prev
    );
  }, [operatorId, setHighlightState]);

  const nodeContent = (
    <div
      className={cn('group relative', nodeVariants({ selected: isSelected }), {
        'shadow-glow': isSelected || isActiveHighlight,
        'shadow-node': !isSelected && !isActiveHighlight,
      })}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={
        {
          borderColor: heatmapColor ?? activeColor,
          backgroundColor: heatmapColor ?? bgColor,
          '--glow-color': 'hsl(var(--primary))',
          ...(fieldColor && isLightColor(fieldColor) ? { color: '#111827' } : {}),
        } as React.CSSProperties
      }
    >
      {data.actions && (
        <DAGItemStatus
          label={data.label}
          pinned={data.actions.pinned}
          filtered={data.actions.filtered}
          className="absolute right-1 top-1 z-20"
        />
      )}
      {data.hasIncoming && (
        <Handle type="target" position={incomingHandlePosition} className="w-2 h-2 opacity-0" />
      )}

      <DataText
        as="div"
        className={cn('text-sm break-words text-center font-normal', {
          'font-bold': data.operationType === 'stage' || isSelected,
          'px-8': data.actions?.pinned && data.actions.filtered,
          'px-5': data.actions && data.actions.pinned !== data.actions.filtered,
        })}
      >
        {resolvedLabel}
      </DataText>
      {formattedColorFieldValue !== null && (
        <div
          className="text-xs text-center mt-0.5"
          style={{
            color: fieldColor
              ? isLightColor(fieldColor)
                ? withOpacity(BLACK, 0.5)
                : withOpacity(WHITE, 0.65)
              : undefined,
          }}
        >
          {formattedColorFieldValue}
        </div>
      )}

      {data.flowBarVisible && operatorId && <NodeFlowBar operatorId={operatorId} />}

      {data.actions && (
        <DAGItemActions
          label={data.label}
          pinned={data.actions.pinned}
          filtered={data.actions.filtered}
          divided={false}
          backgroundColor={heatmapColor ?? activeColor}
          previewOpen={activePreviewId === previewId}
          onOpenChange={open => setActivePreviewId(open ? previewId : null)}
          className="-mx-4 -mb-2 mt-2 w-[calc(100%+2rem)] overflow-hidden rounded-b-md"
          previewContent={
            <OperatorDetailsBlock
              operator={data.actions.details}
              quantitySpecs={data.quantitySpecs}
              isOpen={id => !closedPreviewOperatorIds.has(id)}
              onOpenChange={(id, open) =>
                setClosedPreviewOperatorIds(current => {
                  const next = new Set(current);
                  if (open) {
                    next.delete(id);
                  } else {
                    next.add(id);
                  }
                  return next;
                })
              }
            />
          }
          onTogglePin={data.actions.onTogglePin}
          onToggleFilter={data.actions.onToggleFilter}
        />
      )}

      {data.hasOutgoing && (
        <Handle type="source" position={outgoingHandlePosition} className="w-2 h-2 opacity-0" />
      )}
    </div>
  );

  return <div className={cn(opacityClass, 'z-10')}>{nodeContent}</div>;
});

QueryPlanNode.displayName = 'QueryPlanNode';
