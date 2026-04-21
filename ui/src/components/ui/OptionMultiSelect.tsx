// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState } from 'react';
import { Check, ChevronDown, Search, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

interface OptionMultiSelectProps {
  label: string;
  triggerText: string;
  options: string[];
  selectedOptionIds: Set<string> | null;
  onToggleOption: (optionId: string) => void;
  onSelectAllOptions: () => void;
  onSelectNoOptions: () => void;
  searchPlaceholder?: string;
  emptyMessage?: string;
  noneSelectedText?: string;
  maxVisibleBadges?: number;
  showSelectedBadges?: boolean;
}

export function OptionMultiSelect({
  label,
  triggerText,
  options,
  selectedOptionIds,
  onToggleOption,
  onSelectAllOptions,
  onSelectNoOptions,
  searchPlaceholder = 'Search options…',
  emptyMessage = 'No options found',
  noneSelectedText = 'None selected',
  maxVisibleBadges = 6,
  showSelectedBadges = true,
}: OptionMultiSelectProps) {
  const [search, setSearch] = useState('');
  const selectedOptions = options.filter(option =>
    selectedOptionIds ? selectedOptionIds.has(option) : true
  );
  const visibleSelectedOptions = selectedOptions.slice(0, maxVisibleBadges);
  const hiddenSelectedCount = Math.max(0, selectedOptions.length - visibleSelectedOptions.length);
  const filteredOptions = search
    ? options.filter(option => option.toLowerCase().includes(search.toLowerCase()))
    : options;

  return (
    <div className="flex items-center gap-1 px-3 py-1.5 border-t border-border/50">
      <span className="text-xs text-muted-foreground shrink-0 mr-1">{label}:</span>
      <Popover
        onOpenChange={open => {
          if (!open) setSearch('');
        }}
      >
        <PopoverTrigger asChild>
          <button className="inline-flex h-7 min-w-36 items-center justify-between gap-2 rounded border border-input bg-background px-2 text-xs text-foreground hover:bg-accent hover:text-accent-foreground transition-colors">
            <span className="truncate">
              {selectedOptions.length > 0
                ? `${triggerText} (${selectedOptions.length})`
                : triggerText}
            </span>
            <ChevronDown className="h-3 w-3 text-muted-foreground shrink-0" />
          </button>
        </PopoverTrigger>
        <PopoverContent className="w-64 p-2" align="start" side="bottom">
          <div className="relative mb-2">
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
            <input
              className="w-full pl-6 pr-2 py-1 text-xs border border-input rounded bg-background text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              placeholder={searchPlaceholder}
              value={search}
              onChange={e => setSearch(e.target.value)}
              autoFocus
            />
          </div>
          <div className="flex gap-2 mb-2 border-b border-border pb-2">
            <button onClick={onSelectAllOptions} className="text-xs text-primary hover:underline">
              All
            </button>
            <button onClick={onSelectNoOptions} className="text-xs text-primary hover:underline">
              None
            </button>
          </div>
          <div className="max-h-52 overflow-y-auto space-y-0.5">
            {filteredOptions.map(option => {
              const checked = selectedOptionIds ? selectedOptionIds.has(option) : true;
              return (
                <button
                  key={option}
                  onClick={() => onToggleOption(option)}
                  className="w-full flex items-center gap-2 px-2 py-1 rounded text-xs font-mono text-left hover:bg-accent hover:text-accent-foreground transition-colors"
                >
                  <span
                    className={cn(
                      'flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded border',
                      checked ? 'bg-primary border-primary text-primary-foreground' : 'border-input'
                    )}
                  >
                    {checked && <Check className="h-2.5 w-2.5" />}
                  </span>
                  {option}
                </button>
              );
            })}
            {filteredOptions.length === 0 && (
              <p className="text-xs text-muted-foreground text-center py-2">{emptyMessage}</p>
            )}
          </div>
        </PopoverContent>
      </Popover>
      {showSelectedBadges && (
        <div className="flex-1 min-w-0">
          {selectedOptions.length === 0 ? (
            <span className="text-xs text-muted-foreground italic">{noneSelectedText}</span>
          ) : (
            <div className="flex flex-wrap items-center gap-1">
              {visibleSelectedOptions.map(option => (
                <span
                  key={option}
                  className="inline-flex items-center gap-0.5 text-xs font-mono px-1.5 py-0 rounded border bg-primary/10 border-primary/40 text-data whitespace-nowrap"
                >
                  {option}
                  <span
                    role="button"
                    tabIndex={-1}
                    onClick={e => {
                      e.stopPropagation();
                      onToggleOption(option);
                    }}
                    className="ml-0.5 rounded-sm focus:outline-none"
                    aria-label={`Remove ${option}`}
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
      )}
    </div>
  );
}
