// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useState } from 'react';
import { ChevronUp, ChevronDown } from 'lucide-react';
import {
  useSelectedNodeData,
  useDataFlowEnabled,
  useDataFlowMeta,
  useDataFlowFrame,
  formatDataFlowValue,
  type DataFlowFrame,
  type DataFlowMeta,
  type DataFlowOperatorFrame,
} from '@quent/hooks';
import { DataText } from '../ui/data-text';
import { thinScrollbarClass } from '../ui/thin-scroll';
import {
  createCapacitiesColorFn,
  createFsmTypeColorFn,
  formatDuration,
  inferFieldFormatter,
  type PaletteTheme,
} from '@quent/utils';

const ColorDot = ({ color }: { color: string }) => (
  <span className="inline-block h-2 w-2 rounded-sm shrink-0" style={{ backgroundColor: color }} />
);

/**
 * State × dimension matrix of the data-flow distribution for the selected
 * operator at the playhead's bin. Values are span-weighted per-bin averages
 * ("during this bin"), so fractional counts are expected. Columns are
 * filtered to the SELECTED dimension keys (tiers) — deselected tiers are
 * zero in the frame anyway, so hiding their columns loses nothing.
 */
const DataFlowMatrix = ({
  meta,
  frame,
  operatorFrame,
  isDark,
}: {
  meta: DataFlowMeta;
  frame: DataFlowFrame;
  operatorFrame: DataFlowOperatorFrame;
  isDark: boolean;
}) => {
  const paletteTheme: PaletteTheme = isDark ? 'dark' : 'light';
  const allDimensionKeys = meta.decl.dimension_keys;
  // Keep original decl-order indices — the frame's matrix/byDimension are
  // indexed by declaration order, not by the filtered column order.
  const dimensionColumns = useMemo(
    () =>
      allDimensionKeys
        .map((key, index) => ({ key, index }))
        .filter(({ key }) => meta.dimensionSelection.has(key.key)),
    [allDimensionKeys, meta.dimensionSelection]
  );
  const stateColor = useMemo(
    () =>
      createFsmTypeColorFn(meta.fsmType ? { [meta.fsmType.name]: meta.fsmType } : {}, paletteTheme),
    [meta, paletteTheme]
  );
  const dimensionColor = useMemo(
    () =>
      createCapacitiesColorFn(
        allDimensionKeys.map(k => k.key),
        paletteTheme
      ),
    [allDimensionKeys, paletteTheme]
  );

  const fmt = (value: number) => formatDataFlowValue(value, frame.measure, meta);
  const measureDecl = meta.decl.measures.find(m => m.name === frame.measure);

  return (
    <div className="pt-1.5">
      <div className="text-xs font-medium">
        Data flow @ <DataText>{formatDuration(frame.timeS * 1000)}</DataText>
        <span className="text-muted-foreground font-normal">
          {' '}
          · {measureDecl?.display_name ?? frame.measure} during this bin
        </span>
      </div>
      <table className="mt-1 text-xs w-full border-separate border-spacing-0">
        <thead>
          <tr>
            <th className="text-left font-normal text-muted-foreground pr-2">
              {meta.decl.dimension_name}
            </th>
            {dimensionColumns.map(({ key: k }) => (
              <th key={k.key} className="text-right font-normal text-muted-foreground px-1.5">
                <span className="inline-flex items-center gap-1">
                  <ColorDot color={dimensionColor(k.key)} />
                  <DataText>{k.display_name}</DataText>
                </span>
              </th>
            ))}
            <th className="text-right font-medium text-muted-foreground pl-1.5">Total</th>
          </tr>
        </thead>
        <tbody>
          {meta.stateNames.map((state, stateIndex) => (
            <tr key={state}>
              <td className="pr-2">
                <span className="inline-flex items-center gap-1">
                  <ColorDot color={stateColor(state)} />
                  <DataText>{state}</DataText>
                </span>
              </td>
              {dimensionColumns.map(({ key: k, index: dimensionIndex }) => (
                <td key={k.key} className="text-right px-1.5 text-muted-foreground">
                  <DataText>
                    {fmt(operatorFrame.matrix[stateIndex]?.[dimensionIndex] ?? 0)}
                  </DataText>
                </td>
              ))}
              <td className="text-right pl-1.5">
                <DataText>{fmt(operatorFrame.byState[stateIndex] ?? 0)}</DataText>
              </td>
            </tr>
          ))}
          <tr>
            <td className="pr-2 pt-0.5 font-medium">Total</td>
            {dimensionColumns.map(({ key: k, index: dimensionIndex }) => (
              <td key={k.key} className="text-right px-1.5 pt-0.5">
                <DataText>{fmt(operatorFrame.byDimension[dimensionIndex] ?? 0)}</DataText>
              </td>
            ))}
            <td className="text-right pl-1.5 pt-0.5 font-medium">
              <DataText>{fmt(operatorFrame.total)}</DataText>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
};

export const DAGNodeInfoPanel = ({ isDark = false }: { isDark?: boolean }) => {
  const selectedNodeData = useSelectedNodeData();
  const dataFlowEnabled = useDataFlowEnabled();
  const dataFlowMeta = useDataFlowMeta();
  const dataFlowFrame = useDataFlowFrame();
  const [isExpanded, setIsExpanded] = useState(false);

  const operatorFrame =
    dataFlowEnabled && selectedNodeData && dataFlowMeta && dataFlowFrame
      ? dataFlowFrame.perOperator.get(selectedNodeData.nodeId)
      : undefined;

  useEffect(() => {
    setIsExpanded(!!selectedNodeData);
  }, [selectedNodeData?.nodeId]);

  return (
    <div className="border-t bg-card flex-shrink-0">
      <div className="flex items-center justify-between px-4 py-1.5 min-w-0">
        <div className="flex items-center gap-2 min-w-0 overflow-hidden">
          <span className="text-xs text-muted-foreground font-medium flex-shrink-0">
            Operator Details
          </span>
          {selectedNodeData && (
            <>
              <span className="text-muted-foreground text-xs flex-shrink-0">·</span>
              <DataText className="text-xs font-medium truncate">{selectedNodeData.label}</DataText>
              <DataText className="text-xs text-muted-foreground capitalize px-1.5 py-0.5 bg-muted rounded flex-shrink-0">
                {selectedNodeData.operationType}
              </DataText>
            </>
          )}
        </div>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          disabled={!selectedNodeData}
          className="ml-2 rounded p-1 hover:bg-muted transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-auto disabled:hover:bg-transparent flex-shrink-0"
          aria-label="Toggle operator details"
        >
          {isExpanded ? (
            <ChevronDown className="h-3 w-3 text-muted-foreground" />
          ) : (
            <ChevronUp className="h-3 w-3 text-muted-foreground" />
          )}
        </button>
      </div>

      {isExpanded && selectedNodeData && (
        <div className={`border-t px-4 pb-2 h-48 overflow-y-auto ${thinScrollbarClass}`}>
          {operatorFrame && dataFlowMeta && dataFlowFrame && (
            <DataFlowMatrix
              meta={dataFlowMeta}
              frame={dataFlowFrame}
              operatorFrame={operatorFrame}
              isDark={isDark}
            />
          )}
          <div className="flex flex-col gap-1 pr-2 pt-1.5">
            <div className="text-xs flex items-center justify-between">
              <DataText className="capitalize">ID:</DataText>
              <DataText className="text-muted-foreground ml-1 truncate">
                {selectedNodeData.nodeId}
              </DataText>
            </div>
            {selectedNodeData.statistics?.map(({ key, value }) => (
              <div key={key} className="text-xs">
                {Array.isArray(value) ? (
                  <div className="flex items-center justify-between gap-0.5">
                    <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
                    <div className="ml-2 flex flex-col gap-0.5">
                      {value.map((item, i) => (
                        <DataText key={i} className="text-muted-foreground whitespace-pre-line">
                          {item}
                        </DataText>
                      ))}
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center justify-between">
                    <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
                    <DataText className="text-muted-foreground ml-1">
                      {typeof value === 'number' ? inferFieldFormatter(key)(value) : String(value)}
                    </DataText>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
