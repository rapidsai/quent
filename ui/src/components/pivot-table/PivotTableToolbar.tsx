import { useState, useCallback } from 'react';
import { cn } from '@/lib/utils';
import type { AggMode } from './types';

export interface IndexConfigEntry {
  key: string;
  label: React.ReactNode;
  enabled: boolean;
}

export interface PivotTableToolbarProps {
  indexConfig: IndexConfigEntry[];
  isAggregating: boolean;
  aggMode: AggMode;
  allStats: string[];
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
  const [draggedIndex, setDraggedIndex] = useState<string | null>(null);

  const handleDragStart = useCallback((key: string) => setDraggedIndex(key), []);
  const handleDragOver = useCallback(
    (e: React.DragEvent, targetKey: string) => {
      e.preventDefault();
      if (!draggedIndex || draggedIndex === targetKey) return;
      onReorderIndex(draggedIndex, targetKey);
      setDraggedIndex(targetKey);
    },
    [draggedIndex, onReorderIndex]
  );
  const handleDragEnd = useCallback(() => setDraggedIndex(null), []);

  const sortedStats = [...orderedStats].sort((a, b) => {
    const aChecked = selectedStats ? selectedStats.has(a) : true;
    const bChecked = selectedStats ? selectedStats.has(b) : true;
    if (aChecked !== bChecked) return aChecked ? -1 : 1;
    return 0;
  });

  return (
    <>
      <div className="flex items-center gap-2 px-3 py-1.5">
        <span className="text-xs text-muted-foreground shrink-0">Group by:</span>
        {indexConfig.map(({ key, label, enabled }) => (
          <button
            key={key}
            draggable
            onDragStart={() => handleDragStart(key)}
            onDragOver={e => handleDragOver(e, key)}
            onDragEnd={handleDragEnd}
            onClick={() => onToggleIndex(key)}
            className={cn(
              'text-xs px-2 py-0.5 rounded border transition-colors cursor-grab active:cursor-grabbing select-none whitespace-nowrap',
              enabled
                ? 'bg-primary/10 border-primary/40 text-primary'
                : 'bg-muted/50 border-border text-muted-foreground',
              draggedIndex === key && 'opacity-50'
            )}
          >
            {label}
          </button>
        ))}
        {isAggregating && (
          <>
            <span className="text-xs text-muted-foreground shrink-0 ml-2">Show:</span>
            {(['sum', 'mean', 'min', 'max', 'stdev'] as AggMode[]).map(mode => (
              <button
                key={mode}
                onClick={() => onSetAggMode(mode)}
                className={cn(
                  'text-xs px-2 py-0.5 rounded border transition-colors',
                  aggMode === mode
                    ? 'bg-primary/10 border-primary/40 text-primary'
                    : 'bg-muted/50 border-border text-muted-foreground'
                )}
              >
                {mode}
              </button>
            ))}
          </>
        )}
      </div>
      <div className="relative flex items-center gap-1 px-3 py-1.5 border-t border-border/50 group/cols">
        <span className="text-xs text-muted-foreground shrink-0 mr-1">Columns:</span>
        <button
          onClick={onSelectAllStats}
          className="text-xs text-primary hover:underline shrink-0"
        >
          All
        </button>
        <button onClick={onSelectNoStats} className="text-xs text-primary hover:underline shrink-0">
          None
        </button>
        <div className="flex-1 min-w-0 overflow-hidden flex items-center gap-1">
          {sortedStats.map(stat => {
            const checked = selectedStats ? selectedStats.has(stat) : true;
            return (
              <button
                key={stat}
                onClick={() => onToggleStat(stat)}
                className={cn(
                  'text-xs font-mono px-1.5 py-0 rounded border transition-colors whitespace-nowrap shrink-0',
                  checked
                    ? 'bg-primary/10 border-primary/40 text-data'
                    : 'bg-muted/50 border-border text-data/60'
                )}
              >
                {stat}
              </button>
            );
          })}
        </div>
        <span className="shrink-0 text-xs text-muted-foreground cursor-default">
          &hellip;&#x25BE;
        </span>
        <div className="absolute left-0 top-full z-20 w-full bg-card border border-border rounded-b shadow-lg p-2 hidden group-hover/cols:flex flex-wrap gap-1">
          {sortedStats.map(stat => {
            const checked = selectedStats ? selectedStats.has(stat) : true;
            return (
              <button
                key={stat}
                onClick={() => onToggleStat(stat)}
                className={cn(
                  'text-xs font-mono px-1.5 py-0.5 rounded border transition-colors whitespace-nowrap',
                  checked
                    ? 'bg-primary/10 border-primary/40 text-data'
                    : 'bg-muted/50 border-border text-data/60'
                )}
              >
                {stat}
              </button>
            );
          })}
        </div>
      </div>
    </>
  );
}
