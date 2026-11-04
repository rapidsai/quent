// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Pin, Search, X } from 'lucide-react';
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
import { computePinAllPlan } from '@/features/pinned-rows/pinnedRows';
import { MAX_RESOURCE_FILTER_QUERY_LENGTH } from './resourceFilter';

const EMPTY_PINNED_IDS: ReadonlySet<string> = new Set();

interface PinMatchesButtonProps {
  matchIds: readonly string[];
  onPinMany: (ids: string[]) => void;
  pinnedIds: ReadonlySet<string>;
}

function PinMatchesButton({ matchIds, onPinMany, pinnedIds }: PinMatchesButtonProps) {
  const uniqueIds = [...new Set(matchIds)];
  const plan = computePinAllPlan(uniqueIds, pinnedIds);
  const hasPinnedMatches = plan.toPin.length < uniqueIds.length;

  // With nothing left to pin, the button stays as "Pin all" but is grayed out.
  const label =
    hasPinnedMatches && plan.toPin.length > 0
      ? `Pin remaining ${plan.toPin.length}`
      : `Pin all ${uniqueIds.length}`;

  return (
    <Button
      className="h-6 cursor-pointer gap-1 px-2 text-xs"
      disabled={plan.toPin.length === 0}
      onClick={() => onPinMany(plan.toPin)}
      size="xs"
      type="button"
      variant="outline"
    >
      <Pin aria-hidden className="h-3 w-3" />
      {label}
    </Button>
  );
}

interface ResourceFilterSearchProps {
  fsmTypes: string[];
  matchCount: number;
  // Ids of the rows that match, and the ids already pinned. With the pin callback, they
  // turn on the "pin all matches" button. Leave them out to hide it.
  matchIds?: readonly string[];
  pinnedIds?: ReadonlySet<string>;
  onPinMany?: (ids: string[]) => void;
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
  matchIds,
  onFsmTypesChange,
  onPinMany,
  onResourceTypesChange,
  onSearchChange,
  onShowOthersChange,
  pinnedIds,
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

  const pinAction =
    isActive && matchIds && matchIds.length > 0 && onPinMany ? (
      <PinMatchesButton
        matchIds={matchIds}
        onPinMany={onPinMany}
        pinnedIds={pinnedIds ?? EMPTY_PINNED_IDS}
      />
    ) : null;

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
        <div className="flex w-max max-w-full flex-col gap-2">
          <div className="flex items-center gap-1.5">
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
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-3">
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
            </div>
            <div className="flex items-center gap-1.5">
              {isActive && (
                <span
                  aria-live="polite"
                  className="shrink-0 whitespace-nowrap text-xs text-muted-foreground tabular-nums"
                >
                  {matchCount} {matchCount === 1 ? 'match' : 'matches'}
                </span>
              )}
              {pinAction}
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
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
