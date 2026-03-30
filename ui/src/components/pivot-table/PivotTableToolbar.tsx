import { useState, useCallback } from 'react';
import { Check, ChevronDown, Search, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
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
  const [colSearch, setColSearch] = useState('');

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

  const selectedStatsList = sortedStats.filter(s => (selectedStats ? selectedStats.has(s) : true));
  const filteredStats = colSearch
    ? orderedStats.filter(s => s.toLowerCase().includes(colSearch.toLowerCase()))
    : sortedStats;

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
      <div className="flex items-center gap-1 px-3 py-1.5 border-t border-border/50">
        <span className="text-xs text-muted-foreground shrink-0 mr-1">Columns:</span>
        <Popover
          onOpenChange={open => {
            if (!open) setColSearch('');
          }}
        >
          <PopoverTrigger asChild>
            <button className="flex-1 min-w-0 flex items-center gap-1 flex-wrap cursor-pointer rounded border border-transparent hover:border-border/60 px-1.5 py-0.5 transition-colors text-left">
              {selectedStatsList.length === 0 ? (
                <span className="text-xs text-muted-foreground italic">None selected</span>
              ) : (
                selectedStatsList.map(stat => (
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
                      className="ml-0.5 rounded-sm hover:text-destructive focus:outline-none"
                      aria-label={`Remove ${stat}`}
                    >
                      <X className="h-2.5 w-2.5" />
                    </span>
                  </span>
                ))
              )}
              <ChevronDown className="h-3 w-3 text-muted-foreground ml-auto shrink-0" />
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
      </div>
    </>
  );
}
