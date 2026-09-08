// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Search, X } from 'lucide-react';
import { useMemo } from 'react';
import {
  Badge,
  Button,
  Input,
  OptionMultiSelect,
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@quent/components';
import { cn } from '@quent/utils';
import { MAX_RESOURCE_FILTER_QUERY_LENGTH } from './resourceFilter';

interface ResourceFilterSearchProps {
  fsmTypes: string[];
  matchCount: number;
  onFsmTypesChange: (fsmTypes: string[]) => void;
  onResourceTypesChange: (resourceTypes: string[]) => void;
  onSearchChange: (search: string) => void;
  onShowOthersChange: (showOthers: boolean) => void;
  resourceTypes: string[];
  search: string;
  selectedFsmTypes: string[];
  selectedResourceTypes: string[];
  showOthers: boolean;
}

export function ResourceFilterSearch({
  fsmTypes,
  matchCount,
  onFsmTypesChange,
  onResourceTypesChange,
  onSearchChange,
  onShowOthersChange,
  resourceTypes,
  search,
  selectedFsmTypes,
  selectedResourceTypes,
  showOthers,
}: ResourceFilterSearchProps) {
  const isActive =
    search.trim().length > 0 || selectedResourceTypes.length > 0 || selectedFsmTypes.length > 0;
  const selectedFilterCount =
    Number(search.trim().length > 0) + selectedResourceTypes.length + selectedFsmTypes.length;
  const resourceTypeOptions = useMemo(
    () => resourceTypes.map(value => ({ value })),
    [resourceTypes]
  );
  const fsmTypeOptions = useMemo(() => fsmTypes.map(value => ({ value })), [fsmTypes]);
  const toggleValue = (values: string[], value: string): string[] =>
    values.includes(value) ? values.filter(current => current !== value) : [...values, value];

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          aria-label={
            selectedFilterCount > 0
              ? `Resource filters, ${selectedFilterCount} selected ${
                  selectedFilterCount === 1 ? 'filter' : 'filters'
                }`
              : 'Resource filters'
          }
          className="cursor-pointer gap-1 px-1.5"
          size="xs"
          title="Resource filters"
          type="button"
          variant="ghost"
        >
          <Search aria-hidden className="h-3.5 w-3.5" />
          {selectedFilterCount > 0 && (
            <Badge
              aria-label={`${selectedFilterCount} selected ${
                selectedFilterCount === 1 ? 'filter' : 'filters'
              }`}
              className="h-4 min-w-4 px-1 py-0 text-[10px] leading-none"
            >
              {selectedFilterCount}
            </Badge>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        className="w-auto max-w-[calc(100vw-2rem)] border-border/80 bg-popover py-3 text-popover-foreground shadow-lg"
        side="bottom"
      >
        <div className="flex w-max max-w-full items-center gap-1.5">
          <div className="relative w-80 shrink-0">
            <Search
              aria-hidden
              className="pointer-events-none absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2"
            />
            <Input
              aria-label="Search resource and NVTX tree labels"
              className="h-6 rounded-sm py-0 pl-6 pr-6 text-xs"
              maxLength={MAX_RESOURCE_FILTER_QUERY_LENGTH}
              onChange={event => onSearchChange(event.target.value)}
              placeholder="Search resource or NVTX labels..."
              title="Separate search terms with commas to match any term."
              value={search}
            />
            {search.length > 0 && (
              <Button
                aria-label="Clear search"
                className="absolute right-1 top-1/2 h-4 w-4 -translate-y-1/2 p-0"
                onClick={() => onSearchChange('')}
                size="xs"
                type="button"
                variant="ghost"
              >
                <X className="h-3 w-3" />
              </Button>
            )}
          </div>
          <OptionMultiSelect
            ariaLabel="Filter by resource type"
            className="shrink-0 border-0 p-0"
            onSelectAllOptions={() => onResourceTypesChange(resourceTypes)}
            onSelectNoOptions={() => onResourceTypesChange([])}
            onToggleOption={resourceType =>
              onResourceTypesChange(toggleValue(selectedResourceTypes, resourceType))
            }
            options={resourceTypeOptions}
            searchPlaceholder="Search resource types…"
            selectedOptionIds={new Set(selectedResourceTypes)}
            showSelectedBadges={false}
            triggerClassName="h-6 w-36 min-w-0 cursor-pointer"
            triggerText="Resource type"
          />
          <OptionMultiSelect
            ariaLabel="Filter by FSM type"
            className="shrink-0 border-0 p-0"
            onSelectAllOptions={() => onFsmTypesChange(fsmTypes)}
            onSelectNoOptions={() => onFsmTypesChange([])}
            onToggleOption={fsmType => onFsmTypesChange(toggleValue(selectedFsmTypes, fsmType))}
            options={fsmTypeOptions}
            searchPlaceholder="Search FSM types…"
            selectedOptionIds={new Set(selectedFsmTypes)}
            showSelectedBadges={false}
            triggerClassName="h-6 w-32 min-w-0 cursor-pointer"
            triggerText="FSM type"
          />
          <label
            className={cn(
              'flex shrink-0 items-center gap-1 text-xs',
              isActive ? 'cursor-pointer' : 'cursor-not-allowed opacity-50'
            )}
          >
            <input
              checked={showOthers}
              className="h-3 w-3 accent-primary"
              disabled={!isActive}
              onChange={event => onShowOthersChange(event.target.checked)}
              type="checkbox"
            />
            Show All
          </label>
          <Button
            className="h-6 cursor-pointer px-2 text-xs"
            disabled={!isActive}
            onClick={() => {
              onSearchChange('');
              onResourceTypesChange([]);
              onFsmTypesChange([]);
              onShowOthersChange(false);
            }}
            size="xs"
            type="button"
            variant="ghost"
          >
            Clear
          </Button>
          {isActive && (
            <span
              aria-live="polite"
              className="shrink-0 whitespace-nowrap text-xs text-muted-foreground tabular-nums"
            >
              {matchCount} {matchCount === 1 ? 'match' : 'matches'}
            </span>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
