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
import { useTheme, THEME_DARK, THEME_LIGHT } from '@/contexts/ThemeContext';
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
    () => createFsmTypeColorFn(table.query.fsmTypes, isDark ? THEME_DARK : THEME_LIGHT),
    [table.query.fsmTypes, isDark]
  );

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full">
      <ResizablePanel defaultSize="65%" minSize="40%">
        <div className="flex h-full min-h-0 flex-col">
          <QueryToolbar />
          <EntitiesToolbar
            filters={table.filters.values}
            operatorId={table.filters.operatorId}
            operatorOptions={table.filters.operatorOptions}
            entityTypeOptions={table.filters.entityTypeOptions}
            resourceOptions={table.filters.resourceOptions}
            activeFilterCount={table.filters.activeFilterCount}
            hasNonDefaultSettings={table.filters.hasNonDefaultSettings}
            requestPending={table.query.requestPending}
            validationErrors={table.filters.validationErrors}
            invalidFilterFields={table.filters.invalidFilterFields}
            onOperatorChange={table.filters.updateOperator}
            onFiltersChange={table.filters.update}
            onReset={table.filters.reset}
          />
          <EntityResults
            rows={table.query.rows}
            selected={table.selection.selected}
            isError={table.query.isError}
            isLoading={table.query.isLoading}
            error={table.query.error}
            requestPending={table.query.requestPending}
            hasValidationErrors={table.filters.validationErrors.length > 0}
            page={table.pagination.page}
            pageCount={table.pagination.pageCount}
            pageSize={table.filters.values.pageSize}
            paginationDisabled={table.pagination.disabled}
            total={table.pagination.total}
            visibleStart={table.pagination.visibleStart}
            visibleEnd={table.pagination.visibleEnd}
            sortDir={table.filters.values.sortDir}
            stateColorFn={stateColorFn}
            onSelect={fsm =>
              table.selection.setSelected(current => (current?.id === fsm.id ? null : fsm))
            }
            onPageChange={table.pagination.setPage}
            onPageSizeChange={value =>
              table.filters.update({ pageSize: value }, { preserveSelection: true })
            }
            onSortChange={table.filters.updateSortDir}
          />
        </div>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="35%" minSize="20%" collapsible collapsedSize="0%">
        <EntityDetailPanel
          fsm={table.selection.selected}
          resourceLabel={table.resourceLabel}
          operatorLabel={table.operatorLabel}
          stateColorFn={stateColorFn}
          queryBundle={props.queryBundle}
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
