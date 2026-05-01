// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, useContext } from 'react';
import type {
  HoveredStatInfo,
  PivotTableDisplayConfig,
  PivotTableDnDConfig,
  PivotTableInteractionConfig,
  PivotTableRenderConfig,
  PivotedRow,
} from './types';

export interface PivotTableDerivedState {
  hoveredHeaderItemIds: Set<string> | null;
  columnRanges: Map<string, { min: number; max: number }>;
  buildHoveredStatInfo: (statName: string) => HoveredStatInfo | null;
}

export interface PivotTableRenderContextValue {
  interaction: PivotTableInteractionConfig<PivotedRow>;
  renderConfig: PivotTableRenderConfig;
  display: PivotTableDisplayConfig;
  dnd: PivotTableDnDConfig;
  derived: PivotTableDerivedState;
}

const PivotTableRenderContext = createContext<PivotTableRenderContextValue | null>(null);

interface PivotTableRenderProviderProps {
  value: PivotTableRenderContextValue;
  children: React.ReactNode;
}

export function PivotTableRenderProvider({ value, children }: PivotTableRenderProviderProps) {
  return (
    <PivotTableRenderContext.Provider value={value}>{children}</PivotTableRenderContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function usePivotTableRenderContext(): PivotTableRenderContextValue {
  const value = useContext(PivotTableRenderContext);
  if (value == null) {
    throw new Error('usePivotTableRenderContext must be used within PivotTableRenderProvider');
  }
  return value;
}
