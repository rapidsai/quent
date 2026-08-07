// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { LoaderCircle, RotateCcw } from 'lucide-react';
import {
  Button,
  Input,
  SearchableSelect,
  SelectField,
  type SelectFieldOption,
} from '@quent/components';
import { cn } from '@quent/utils';
import type { SortDir } from '@quent/utils';
import type { EntityFilters } from './types';
import { MAX_PAGE_SIZE, normalizePageSize } from './utils';

const SORT_DIR_OPTIONS: SelectFieldOption[] = [
  { value: 'Desc', label: 'Longest first' },
  { value: 'Asc', label: 'Shortest first' },
];

interface EntitiesToolbarProps {
  filters: EntityFilters;
  operatorId: string | null;
  operatorOptions: SelectFieldOption[];
  entityTypeOptions: SelectFieldOption[];
  resourceOptions: SelectFieldOption[];
  activeFilterCount: number;
  hasNonDefaultSettings: boolean;
  requestPending: boolean;
  validationErrors: string[];
  onOperatorChange: (value: string | null) => void;
  onFiltersChange: (
    patch: Partial<EntityFilters>,
    options?: { preserveSelection?: boolean }
  ) => void;
  onReset: () => void;
}

export function EntitiesToolbar({
  filters,
  operatorId,
  operatorOptions,
  entityTypeOptions,
  resourceOptions,
  activeFilterCount,
  hasNonDefaultSettings,
  requestPending,
  validationErrors,
  onOperatorChange,
  onFiltersChange,
  onReset,
}: EntitiesToolbarProps) {
  return (
    <div className="shrink-0 border-b bg-card p-3 flex flex-wrap items-end gap-3">
      <SearchableSelect
        label="Operator"
        className="w-72"
        placeholder="All operators"
        options={operatorOptions}
        value={operatorId}
        onValueChange={onOperatorChange}
      />
      <SelectField
        label="Type"
        className="w-40"
        placeholder="All types"
        options={entityTypeOptions}
        value={filters.entityType ?? ''}
        onValueChange={value => onFiltersChange({ entityType: value })}
      />
      <SearchableSelect
        label="Resource"
        className="w-72"
        placeholder="All resources"
        options={resourceOptions}
        value={filters.resourceId}
        onValueChange={value => onFiltersChange({ resourceId: value })}
      />
      <NumberField
        label="Min usage (s)"
        className="w-28"
        value={filters.minUsageS}
        onChange={value => onFiltersChange({ minUsageS: value })}
      />
      <NumberField
        label="Window start (s)"
        className="w-28"
        value={filters.windowStart}
        onChange={value => onFiltersChange({ windowStart: value })}
      />
      <NumberField
        label="Window end (s)"
        className="w-28"
        value={filters.windowEnd}
        onChange={value => onFiltersChange({ windowEnd: value })}
      />
      <SelectField
        label="Sort"
        className="w-60"
        clearable={false}
        options={SORT_DIR_OPTIONS}
        value={filters.sortDir}
        onValueChange={value =>
          onFiltersChange(
            { sortDir: (value as SortDir | null) ?? 'Desc' },
            { preserveSelection: true }
          )
        }
      />
      <PageSizeField
        value={filters.pageSize}
        onChange={value => onFiltersChange({ pageSize: value }, { preserveSelection: true })}
      />
      <Button variant="outline" size="sm" disabled={!hasNonDefaultSettings} onClick={onReset}>
        <RotateCcw className="mr-1.5 size-3.5" />
        Reset filters
      </Button>
      {activeFilterCount > 0 && (
        <span className="pb-1 text-xs text-muted-foreground">
          {activeFilterCount} active {activeFilterCount === 1 ? 'filter' : 'filters'}
        </span>
      )}
      {requestPending && validationErrors.length === 0 && (
        <span
          role="status"
          aria-live="polite"
          className="flex items-center gap-1 pb-1 text-xs text-muted-foreground"
        >
          <LoaderCircle className="size-3.5 animate-spin" />
          Updating…
        </span>
      )}
      {validationErrors.length > 0 && (
        <div role="alert" className="basis-full text-xs text-destructive">
          {validationErrors.join(' ')}
        </div>
      )}
    </div>
  );
}

function NumberField({
  label,
  value,
  className,
  onChange,
}: {
  label: string;
  value: string;
  className?: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      <Input
        type="number"
        min={0}
        step="any"
        className={cn('h-8', className)}
        value={value}
        onChange={event => onChange(event.target.value)}
      />
    </label>
  );
}

function PageSizeField({
  value,
  onChange,
}: {
  value: number | null;
  onChange: (value: number | null) => void;
}) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      Page size
      <Input
        type="number"
        min={1}
        max={MAX_PAGE_SIZE}
        step={1}
        className="h-8 w-24"
        value={value ?? ''}
        onChange={event => onChange(event.target.value === '' ? null : event.target.valueAsNumber)}
        onBlur={() => onChange(normalizePageSize(value))}
      />
    </label>
  );
}
