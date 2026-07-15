// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import {
  formatDataFlowValue,
  type DataFlowFrame,
  type DataFlowMeta,
  type DataFlowOperatorFrame,
} from '@quent/hooks';
import {
  createCapacitiesColorFn,
  createFsmTypeColorFn,
  formatDuration,
  type PaletteTheme,
} from '@quent/utils';
import { DataText } from '../ui/data-text';
import { ColorDot } from './ColorDot';

/**
 * State × dimension matrix of the data-flow distribution for the selected
 * operator at the playhead's bin. Values are span-weighted per-bin averages
 * ("during this bin"), so fractional counts are expected. Columns are
 * filtered to the SELECTED dimension keys (tiers) — deselected tiers are
 * zero in the frame anyway, so hiding their columns loses nothing.
 */
export const DataFlowMatrix = ({
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
