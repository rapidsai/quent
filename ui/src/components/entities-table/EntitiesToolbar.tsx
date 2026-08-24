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
    <div className="shrink-0 border-b bg-card px-3 py-2.5">
      <div className="flex flex-wrap items-end gap-x-3 gap-y-3">
        {/* Entity filters */}
        <div className="flex items-end gap-2.5">
          <FieldWrapper label="Operator" className="w-64">
            <SearchableSelect
              ariaLabel="Operator"
              placeholder="All operators"
              options={operatorOptions}
              value={operatorId}
              onValueChange={onOperatorChange}
            />
          </FieldWrapper>
          <FieldWrapper label="Type" className="w-36">
            <SelectField
              ariaLabel="Type"
              placeholder="All types"
              options={entityTypeOptions}
              value={filters.entityType ?? ''}
              onValueChange={value => onFiltersChange({ entityType: value })}
              triggerClassName="h-8"
            />
          </FieldWrapper>
          <FieldWrapper label="Resource" className="w-64">
            <SearchableSelect
              ariaLabel="Resource"
              placeholder="All resources"
              options={resourceOptions}
              value={filters.resourceId}
              onValueChange={value => onFiltersChange({ resourceId: value })}
            />
          </FieldWrapper>
        </div>

        {/* Time filters */}
        <div className="flex items-end gap-2.5">
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
        </div>

        {/* Display settings */}
        <div className="flex items-end gap-2.5">
          <FieldWrapper label="Sort" className="w-44">
            <SelectField
              ariaLabel="Sort"
              clearable={false}
              options={SORT_DIR_OPTIONS}
              value={filters.sortDir}
              onValueChange={value =>
                onFiltersChange(
                  { sortDir: (value as SortDir | null) ?? 'Desc' },
                  { preserveSelection: true }
                )
              }
              triggerClassName="h-8"
            />
          </FieldWrapper>
        </div>

        {/* Actions */}
        <div className="ml-auto flex items-end gap-3">
          <Button variant="outline" size="sm" disabled={!hasNonDefaultSettings} onClick={onReset}>
            <RotateCcw className="mr-1.5 size-3.5" />
            Reset
          </Button>
          {activeFilterCount > 0 && (
            <span className="text-xs text-muted-foreground">
              {activeFilterCount} active {activeFilterCount === 1 ? 'filter' : 'filters'}
            </span>
          )}
          {requestPending && validationErrors.length === 0 && (
            <span
              role="status"
              aria-live="polite"
              className="flex items-center gap-1 text-xs text-muted-foreground"
            >
              <LoaderCircle className="size-3.5 animate-spin" />
              Updating…
            </span>
          )}
        </div>
      </div>

      {validationErrors.length > 0 && (
        <div role="alert" className="mt-2 text-xs text-destructive">
          {validationErrors.join(' ')}
        </div>
      )}
    </div>
  );
}

function FieldWrapper({
  label,
  children,
  className,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={cn('flex flex-col gap-1', className)}>
      <span className="text-xs text-muted-foreground">{label}</span>
      {children}
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
    <label className={cn('flex flex-col gap-1', className)}>
      <span className="text-xs text-muted-foreground">{label}</span>
      <Input
        type="number"
        min={0}
        step="any"
        className="h-8 text-xs"
        value={value}
        onChange={event => onChange(event.target.value)}
      />
    </label>
  );
}
