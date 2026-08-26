// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { LoaderCircle, RotateCcw } from 'lucide-react';
import {
  Button,
  RangeSliderField,
  SearchableSelect,
  SelectField,
  SliderField,
  type SelectFieldOption,
} from '@quent/components';
import { cn } from '@quent/utils';
import type { EntityNumberFilterField } from './utils';
import type { EntityFilters } from './types';

const FILTER_ERRORS_ID = 'entities-filter-errors';

interface EntitiesToolbarProps {
  filters: EntityFilters;
  durationS: number;
  operatorId: string | null;
  operatorOptions: SelectFieldOption[];
  entityTypeOptions: SelectFieldOption[];
  resourceOptions: SelectFieldOption[];
  activeFilterCount: number;
  hasNonDefaultSettings: boolean;
  requestPending: boolean;
  validationErrors: string[];
  invalidFilterFields: Set<EntityNumberFilterField>;
  onOperatorChange: (value: string | null) => void;
  onFiltersChange: (
    patch: Partial<EntityFilters>,
    options?: { preserveSelection?: boolean }
  ) => void;
  onReset: () => void;
}

export function EntitiesToolbar({
  filters,
  durationS,
  operatorId,
  operatorOptions,
  entityTypeOptions,
  resourceOptions,
  activeFilterCount,
  hasNonDefaultSettings,
  requestPending,
  validationErrors,
  invalidFilterFields,
  onOperatorChange,
  onFiltersChange,
  onReset,
}: EntitiesToolbarProps) {
  const sliderMax = Math.max(durationS, 0);
  return (
    <div className="shrink-0 border-b bg-card px-3 py-2.5">
      <div className="flex flex-wrap items-end justify-between gap-x-4 gap-y-3">
        {/* Filters */}
        <div className="flex flex-wrap items-end gap-x-3 gap-y-3">
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
          <SliderField
            label="Min usage (s)"
            className="w-32"
            min={0}
            max={sliderMax}
            value={filters.minUsageS}
            invalid={invalidFilterFields.has('minUsageS')}
            errorMessageId={FILTER_ERRORS_ID}
            onChange={value => onFiltersChange({ minUsageS: value })}
          />
          <RangeSliderField
            label="Window (s)"
            startLabel="Window start (s)"
            endLabel="Window end (s)"
            className="w-56"
            min={0}
            max={sliderMax}
            startValue={filters.windowStart}
            endValue={filters.windowEnd}
            invalidStart={invalidFilterFields.has('windowStart')}
            invalidEnd={invalidFilterFields.has('windowEnd')}
            errorMessageId={FILTER_ERRORS_ID}
            onStartChange={value => onFiltersChange({ windowStart: value })}
            onEndChange={value => onFiltersChange({ windowEnd: value })}
          />
        </div>

        {/* Actions */}
        <div className="flex items-end gap-3">
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
          {activeFilterCount > 0 && (
            <span className="text-xs text-muted-foreground">
              {activeFilterCount} active {activeFilterCount === 1 ? 'filter' : 'filters'}
            </span>
          )}
          <Button variant="outline" size="sm" disabled={!hasNonDefaultSettings} onClick={onReset}>
            <RotateCcw className="mr-1.5 size-3.5" />
            Reset
          </Button>
        </div>
      </div>

      {validationErrors.length > 0 && (
        <div id={FILTER_ERRORS_ID} role="alert" className="mt-2 text-xs text-destructive">
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
    <label className={cn('flex flex-col gap-1', className)}>
      <span className="text-xs text-muted-foreground">{label}</span>
      {children}
    </label>
  );
}
