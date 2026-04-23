// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import { Check, ChevronDown, Search, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
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

  const isSelected = (option: string): boolean =>
    selectedOptionIds ? selectedOptionIds.has(option) : true;

  const selectedOptions = useMemo(() => options.filter(isSelected), [options, selectedOptionIds]);
  const visibleSelectedOptions = selectedOptions.slice(0, maxVisibleBadges);
  const hiddenSelectedCount = Math.max(0, selectedOptions.length - visibleSelectedOptions.length);

  const filteredOptions = useMemo(() => {
    if (!search) return options;
    const needle = search.toLowerCase();
    return options.filter(option => option.toLowerCase().includes(needle));
  }, [options, search]);

  return (
    <div className="flex items-center gap-1 px-3 py-1.5 border-t border-border/50">
      <span className="text-xs text-muted-foreground shrink-0 mr-1">{label}:</span>
      <Popover
        onOpenChange={open => {
          if (!open) setSearch('');
        }}
      >
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            role="combobox"
            className="h-7 min-w-36 justify-between gap-2 px-2 text-xs font-normal"
          >
            <span className="truncate">
              {selectedOptions.length > 0
                ? `${triggerText} (${selectedOptions.length})`
                : triggerText}
            </span>
            <ChevronDown className="text-muted-foreground shrink-0 opacity-70" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-64 p-2" align="start" side="bottom">
          <div className="relative mb-2">
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 size-3 text-muted-foreground pointer-events-none" />
            <Input
              type="search"
              className="h-7 pl-7 pr-2 text-xs md:text-xs"
              placeholder={searchPlaceholder}
              value={search}
              onChange={e => setSearch(e.target.value)}
              autoFocus
            />
          </div>
          <div className="flex gap-1 mb-2 border-b border-border pb-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={onSelectAllOptions}
              className="h-6 px-2 text-xs text-primary hover:text-primary"
            >
              All
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={onSelectNoOptions}
              className="h-6 px-2 text-xs text-primary hover:text-primary"
            >
              None
            </Button>
          </div>
          <div role="listbox" aria-multiselectable className="max-h-52 overflow-y-auto space-y-0.5">
            {filteredOptions.map(option => {
              const checked = isSelected(option);
              return (
                <button
                  key={option}
                  type="button"
                  role="option"
                  aria-selected={checked}
                  data-state={checked ? 'checked' : 'unchecked'}
                  onClick={() => onToggleOption(option)}
                  className={cn(
                    'relative flex w-full cursor-default select-none items-center gap-2 rounded-sm px-2 py-1 text-xs font-mono outline-none',
                    'transition-colors hover:bg-accent hover:text-accent-foreground',
                    'focus-visible:bg-accent focus-visible:text-accent-foreground'
                  )}
                >
                  <span
                    aria-hidden
                    className={cn(
                      'flex size-3.5 shrink-0 items-center justify-center rounded-sm border transition-colors',
                      checked
                        ? 'bg-primary border-primary text-primary-foreground'
                        : 'border-input bg-background'
                    )}
                  >
                    {checked && <Check className="size-2.5" strokeWidth={3} />}
                  </span>
                  <span className="truncate">{option}</span>
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
                <Badge
                  key={option}
                  variant="outline"
                  className="px-1.5 py-0 font-mono text-data bg-primary/10 border-primary/40 hover:bg-primary/15"
                >
                  <span className="truncate">{option}</span>
                  <button
                    type="button"
                    onClick={e => {
                      e.stopPropagation();
                      onToggleOption(option);
                    }}
                    aria-label={`Remove ${option}`}
                    className="ml-0.5 rounded-sm opacity-70 hover:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  >
                    <X className="size-2.5" />
                  </button>
                </Badge>
              ))}
              {hiddenSelectedCount > 0 && (
                <Badge variant="outline" className="px-1.5 py-0 bg-muted/40 text-muted-foreground">
                  +{hiddenSelectedCount} more
                </Badge>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
