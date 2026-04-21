// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback } from 'react';
import { Check, ChevronDown, Search, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
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
  const [colSearch, setColSearch] = useState('');

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

  const selectedStatsList = orderedStats.filter(s => (selectedStats ? selectedStats.has(s) : true));
  const maxVisibleBadges = 6;
  const visibleSelectedStats = selectedStatsList.slice(0, maxVisibleBadges);
  const hiddenSelectedCount = Math.max(0, selectedStatsList.length - visibleSelectedStats.length);
  const filteredStats = colSearch
    ? orderedStats.filter(s => s.toLowerCase().includes(colSearch.toLowerCase()))
    : orderedStats;

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
                'text-xs px-2 py-0.5 rounded border transition-colors cursor-grab active:cursor-grabbing select-none whitespace-nowrap',
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
        {isAggregating && (
          <div className="ml-auto flex items-center gap-2">
            <span className="text-xs text-muted-foreground shrink-0">Aggregate:</span>
            <Select value={aggMode} onValueChange={value => onSetAggMode(value as AggMode)}>
              <SelectTrigger className="h-7 w-[110px] rounded border border-input px-2 py-0 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent align="start">
                {(['sum', 'mean', 'min', 'max', 'stdev'] as AggMode[]).map(mode => (
                  <SelectItem key={mode} value={mode} className="text-xs">
                    {mode}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}
      </div>
      <div className="flex items-center gap-1 px-3 py-1.5 border-t border-border/50">
        <span className="text-xs text-muted-foreground shrink-0 mr-1">Columns:</span>
        <Popover
          onOpenChange={open => {
            if (!open) setColSearch('');
          }}
        >
          <PopoverTrigger asChild>
            <button className="inline-flex h-7 min-w-36 items-center justify-between gap-2 rounded border border-input bg-background px-2 text-xs text-foreground hover:bg-accent hover:text-accent-foreground transition-colors">
              <span className="truncate">
                {selectedStatsList.length > 0
                  ? `Select Columns (${selectedStatsList.length})`
                  : 'Select Columns'}
              </span>
              <ChevronDown className="h-3 w-3 text-muted-foreground shrink-0" />
            </button>
          </PopoverTrigger>
          <PopoverContent className="w-64 p-2" align="start" side="bottom">
            <div className="relative mb-2">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
              <input
                className="w-full pl-6 pr-2 py-1 text-xs border border-input rounded bg-background text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                placeholder="Search columns…"
                value={colSearch}
                onChange={e => setColSearch(e.target.value)}
                autoFocus
              />
            </div>
            <div className="flex gap-2 mb-2 border-b border-border pb-2">
              <button onClick={onSelectAllStats} className="text-xs text-primary hover:underline">
                All
              </button>
              <button onClick={onSelectNoStats} className="text-xs text-primary hover:underline">
                None
              </button>
            </div>
            <div className="max-h-52 overflow-y-auto space-y-0.5">
              {filteredStats.map(stat => {
                const checked = selectedStats ? selectedStats.has(stat) : true;
                return (
                  <button
                    key={stat}
                    onClick={() => onToggleStat(stat)}
                    className="w-full flex items-center gap-2 px-2 py-1 rounded text-xs font-mono text-left hover:bg-accent hover:text-accent-foreground transition-colors"
                  >
                    <span
                      className={cn(
                        'flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded border',
                        checked
                          ? 'bg-primary border-primary text-primary-foreground'
                          : 'border-input'
                      )}
                    >
                      {checked && <Check className="h-2.5 w-2.5" />}
                    </span>
                    {stat}
                  </button>
                );
              })}
              {filteredStats.length === 0 && (
                <p className="text-xs text-muted-foreground text-center py-2">No columns found</p>
              )}
            </div>
          </PopoverContent>
        </Popover>
        <div className="flex-1 min-w-0">
          {selectedStatsList.length === 0 ? (
            <span className="text-xs text-muted-foreground italic">None selected</span>
          ) : (
            <div className="flex flex-wrap items-center gap-1">
              {visibleSelectedStats.map(stat => (
                <span
                  key={stat}
                  className="inline-flex items-center gap-0.5 text-xs font-mono px-1.5 py-0 rounded border bg-primary/10 border-primary/40 text-data whitespace-nowrap"
                >
                  {stat}
                  <span
                    role="button"
                    tabIndex={-1}
                    onClick={e => {
                      e.stopPropagation();
                      onToggleStat(stat);
                    }}
                    className="ml-0.5 rounded-sm focus:outline-none"
                    aria-label={`Remove ${stat}`}
                  >
                    <X className="h-2.5 w-2.5" />
                  </span>
                </span>
              ))}
              {hiddenSelectedCount > 0 && (
                <span className="inline-flex items-center text-xs px-1.5 py-0 rounded border bg-muted/40 border-border text-muted-foreground whitespace-nowrap">
                  +{hiddenSelectedCount} more
                </span>
              )}
            </div>
          )}
        </div>
      </div>
    </>
  );
}
