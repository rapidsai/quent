// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  QueryToolbar,
} from '@quent/components';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { createFsmTypeColorFn } from '@quent/utils';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import { EntityDetailPanel } from './EntityDetailPanel';
import { EntityResults } from './EntityResults';
import { EntitiesToolbar } from './EntitiesToolbar';
import { useEntityTable } from './useEntityTable';

interface EntitiesTableProps {
  engineId: string;
  queryId: string;
  queryBundle: QueryBundle<EntityRef>;
}

export function EntitiesTable(props: EntitiesTableProps) {
  const table = useEntityTable(props);
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;
  const stateColorFn = useMemo(
    () => createFsmTypeColorFn(table.fsmTypes, isDark ? 'dark' : 'light'),
    [table.fsmTypes, isDark]
  );

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full">
      <ResizablePanel defaultSize="65%" minSize="40%">
        <div className="flex h-full min-h-0 flex-col">
          <QueryToolbar />
          <EntitiesToolbar
            filters={table.filters}
            operatorId={table.operatorId}
            operatorOptions={table.operatorOptions}
            entityTypeOptions={table.entityTypeOptions}
            resourceOptions={table.resourceOptions}
            activeFilterCount={table.activeFilterCount}
            hasNonDefaultSettings={table.hasNonDefaultSettings}
            requestPending={table.requestPending}
            validationErrors={table.validationErrors}
            onOperatorChange={table.updateOperator}
            onFiltersChange={table.updateFilters}
            onReset={table.resetFilters}
          />
          <EntityResults
            rows={table.rows}
            selected={table.selected}
            isError={table.isError}
            isLoading={table.isLoading}
            error={table.error}
            requestPending={table.requestPending}
            hasValidationErrors={table.validationErrors.length > 0}
            page={table.page}
            pageCount={table.pageCount}
            pageSize={table.filters.pageSize}
            paginationDisabled={table.paginationDisabled}
            total={table.total}
            visibleStart={table.visibleStart}
            visibleEnd={table.visibleEnd}
            sortDir={table.filters.sortDir}
            stateColorFn={stateColorFn}
            onSelect={table.setSelected}
            onPageChange={table.setPage}
            onPageSizeChange={value =>
              table.updateFilters({ pageSize: value }, { preserveSelection: true })
            }
            onSortChange={table.updateSortDir}
          />
        </div>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="35%" minSize="20%" collapsible collapsedSize="0%">
        <EntityDetailPanel
          fsm={table.selected}
          resourceLabel={table.resourceLabel}
          operatorLabel={table.operatorLabel}
          stateColorFn={stateColorFn}
          queryBundle={props.queryBundle}
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
