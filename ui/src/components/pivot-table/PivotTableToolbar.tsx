// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback } from 'react';
import { cn } from '@quent/utils';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  OptionMultiSelect,
} from '@quent/components';
import type { AggMode } from './types';
import { useColumnDragDrop } from './useColumnDragDrop';

export interface IndexConfigEntry {
  key: string;
  label: React.ReactNode;
  enabled: boolean;
}

export interface PivotTableToolbarProps {
  indexConfig: IndexConfigEntry[];
  isAggregating: boolean;
  aggMode: AggMode;
  orderedStats: string[];
  selectedStats: Set<string> | null;
  onToggleIndex: (key: string) => void;
  onReorderIndex: (fromKey: string, toKey: string) => void;
  onSetAggMode: (mode: AggMode) => void;
  onToggleStat: (stat: string) => void;
  onSelectAllStats: () => void;
  onSelectNoStats: () => void;
}

export function PivotTableToolbar({
  indexConfig,
  isAggregating,
  aggMode,
  orderedStats,
  selectedStats,
  onToggleIndex,
  onReorderIndex,
  onSetAggMode,
  onToggleStat,
  onSelectAllStats,
  onSelectNoStats,
}: PivotTableToolbarProps) {
  const commitDrop = useCallback(
    (fromKey: string, toKey: string, position: 'before' | 'after') => {
      if (fromKey === toKey) return;
      const keys = indexConfig.map(entry => entry.key);
      const fromIndex = keys.indexOf(fromKey);
      const targetIndex = keys.indexOf(toKey);
      if (fromIndex < 0 || targetIndex < 0) return;

      let anchorKey = toKey;
      if (position === 'before' && fromIndex < targetIndex) {
        anchorKey = keys[targetIndex - 1] ?? toKey;
      } else if (position === 'after' && fromIndex > targetIndex) {
        anchorKey = keys[targetIndex + 1] ?? toKey;
      }
      if (anchorKey === fromKey) return;
      onReorderIndex(fromKey, anchorKey);
    },
    [indexConfig, onReorderIndex]
  );

  const dragDrop = useColumnDragDrop({ onDropCommit: commitDrop });

  return (
    <>
      <div className="flex items-center gap-2 px-3 py-1.5">
        <span className="text-xs text-muted-foreground shrink-0">Group by:</span>
        {indexConfig.map(({ key, label, enabled }) => {
          const dropPosition = dragDrop.getDropTargetPosition(key);
          const dropIndicatorStyle = dropPosition
            ? {
                boxShadow:
                  dropPosition === 'before'
                    ? 'inset 3px 0 0 hsl(var(--primary))'
                    : 'inset -3px 0 0 hsl(var(--primary))',
              }
            : undefined;
          return (
            <button
              key={key}
              draggable
              onDragStart={e => dragDrop.handleDragStart(e, key)}
              onDragOver={e => dragDrop.handleDragOver(e, key)}
              onDragLeave={e => dragDrop.handleDragLeave(e, key)}
              onDrop={e => dragDrop.handleDrop(e, key)}
              onDragEnd={dragDrop.handleDragEnd}
              onClick={() => onToggleIndex(key)}
              className={cn(
                'text-xs px-2 py-0.5 rounded border transition-colors cursor-grab active:cursor-grabbing select-none whitespace-nowrap h-full',
                {
                  'bg-primary/10 border-primary/40 text-primary': enabled,
                  'bg-muted/50 border-border text-muted-foreground': !enabled,
                  'opacity-45': dragDrop.draggedId === key,
                }
              )}
              style={dropIndicatorStyle}
            >
              {label}
            </button>
          );
        })}
        <div className="ml-auto flex items-center gap-2">
          <span className="text-xs text-muted-foreground shrink-0">Aggregate:</span>
          <Select
            value={isAggregating ? aggMode : '--'}
            onValueChange={value => onSetAggMode(value as AggMode)}
            disabled={!isAggregating}
          >
            <SelectTrigger className="h-7 w-[110px] rounded border border-input px-2 py-0 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent align="start">
              {!isAggregating && (
                <SelectItem value="--" className="text-xs" disabled>
                  --
                </SelectItem>
              )}
              {(['sum', 'mean', 'min', 'max', 'stdev'] as AggMode[]).map(mode => (
                <SelectItem key={mode} value={mode} className="text-xs">
                  {mode}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>
      <OptionMultiSelect
        label="Columns"
        triggerText="Select Columns"
        options={orderedStats}
        selectedOptionIds={selectedStats}
        onToggleOption={onToggleStat}
        onSelectAllOptions={onSelectAllStats}
        onSelectNoOptions={onSelectNoStats}
        searchPlaceholder="Search columns…"
        emptyMessage="No columns found"
        noneSelectedText="None selected"
      />
    </>
  );
}
