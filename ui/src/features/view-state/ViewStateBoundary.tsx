// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useStore } from 'jotai';
import {
  LONG_ENTITY_DENSITY_DEFAULT,
  useHydrateSelectedPlanId,
  useReadZoomRange,
  useSerializableViewState,
  useSetDebouncedZoomRange,
  useSetZoomRange,
  showNestedChartRowsAtom,
} from '@quent/hooks';
import { DAG_LAYOUT_DIRECTION, SCALE_TYPE, NODE_LABEL_FIELD, type Operator } from '@quent/utils';
import {
  entitiesTableStateAtom,
  type EntitiesTableState,
  type EntityFilters,
} from '@/atoms/entitiesTable';
import {
  defaultExpandedIdsAtom,
  expandedIdsAtom,
  pinnedCollapsedAtom,
  pinnedIdsAtom,
  pinnedPanelSizeAtom,
  resourceFilterAtom,
  rootResourceTypeAtom,
  selectedNvtxCategoriesAtom,
  selectedNvtxDomainAtom,
  selectedFsmTypesAtom,
  selectedTypesAtom,
  sharedCollapsedIdsAtom,
} from '@/atoms/resourceTree';
import {
  EMPTY_RESOURCE_FILTER,
  isResourceFilterActive,
} from '@/features/resource-filter/resourceFilter';
import {
  DEFAULT_OPERATOR_TABLE_ENABLED,
  OPERATOR_TABLE_INDEX_ORDER,
  OPERATOR_TABLE_PERSIST_KEY,
} from '@/components/operator-table/types';
import {
  DEFAULT_PAGE_SIZE,
  SORT_DESC,
  defaultEntityFilters,
  normalizePageSize,
  parseOptionalNumber,
  validateEntityFilters,
} from '@/components/entities-table/utils';
import { decodeViewState, encodeViewState, type ViewStateCodecOptions } from './viewState.codec';
import {
  ENCODED_VIEW_STATE_FAILURE,
  ViewStateContext,
  type ViewStateContextValue,
  type ViewStateIntakeStatus,
  type EncodedViewStateResult,
} from './viewState.context';
import { readViewStateFields, type ViewStateFields } from './viewState.fields';
import { normalizeZoomRange, resolveCapturedZoomRange } from './viewState.normalize';
import {
  OperatorGroupSchema,
  type ViewStateEntitiesState,
  type ViewStateV3,
  type ViewStateTab,
} from './viewState.schema';

interface ViewStateBoundaryProps {
  children: ReactNode;
  engineId: string;
  queryId?: string;
  activeTab?: ViewStateTab;
  initialPlanId: string;
  durationSeconds: number;
  defaultRootResourceType?: string | null;
  operators: readonly Operator[];
  encodedState?: string;
  /** Size limit of the source `encodedState` came from. Defaults to the shared-link limit. */
  maxEncodedLength?: number;
  isQueryReady: boolean;
}

type CaptureStateResult =
  | { ok: true; value: ViewStateV3 }
  | {
      ok: false;
      reason: typeof ENCODED_VIEW_STATE_FAILURE.UNAVAILABLE;
      message: string;
      messages: readonly string[];
    };
type ViewFailure = Extract<CaptureStateResult, { ok: false }>;

type IntakeState = {
  initialExpandedResourceIds: readonly string[] | null;
  initialZoomRange: NonNullable<ViewStateFields['zoomRange']> | null;
  fields: ViewStateFields | null;
  isResolved: boolean;
  status: ViewStateIntakeStatus;
};

function hasKeys(value: object): boolean {
  return Object.keys(value).length > 0;
}

function captureFailure(message: string): ViewFailure {
  return {
    ok: false,
    reason: ENCODED_VIEW_STATE_FAILURE.UNAVAILABLE,
    message,
    messages: [message],
  };
}

function isOperatorGroup(value: string): value is (typeof OperatorGroupSchema.options)[number] {
  return OperatorGroupSchema.validate(value);
}

function hydrateEntitiesState(
  state: ViewStateEntitiesState,
  durationSeconds: number
): EntitiesTableState {
  const filters: EntityFilters = {
    ...defaultEntityFilters(durationSeconds),
    entityType: state.entityType ?? null,
    resourceId: state.resourceId ?? null,
    minUsageS: state.minUsageS === undefined ? '' : String(state.minUsageS),
    windowStart: state.window ? String(state.window.start) : '0',
    windowEnd: state.window ? String(state.window.end) : String(durationSeconds),
    sortDir: state.sortDir ?? SORT_DESC,
    pageSize: state.pageSize ?? DEFAULT_PAGE_SIZE,
  };

  return {
    filters,
    page: state.page ?? 0,
    selected: null,
    selectedEntityId: state.selectedEntityId ?? null,
  };
}

function captureEntitiesState(
  tableState: EntitiesTableState,
  durationSeconds: number
): ViewStateEntitiesState | null {
  const filters = tableState.filters ?? defaultEntityFilters(durationSeconds);
  if (validateEntityFilters(filters).errors.length > 0) {
    return null;
  }

  const state: ViewStateEntitiesState = {};
  if (filters.entityType) {
    state.entityType = filters.entityType;
  }
  if (filters.resourceId) {
    state.resourceId = filters.resourceId;
  }
  const minUsageS = parseOptionalNumber(filters.minUsageS);
  if (minUsageS !== null) {
    state.minUsageS = minUsageS;
  }
  const windowStart = parseOptionalNumber(filters.windowStart) ?? 0;
  const windowEnd = parseOptionalNumber(filters.windowEnd) ?? durationSeconds;
  if (windowStart !== 0 || windowEnd !== durationSeconds) {
    state.window = { start: windowStart, end: windowEnd };
  }
  if (filters.sortDir !== SORT_DESC) {
    state.sortDir = filters.sortDir;
  }
  const pageSize = normalizePageSize(filters.pageSize);
  if (pageSize !== DEFAULT_PAGE_SIZE) {
    state.pageSize = pageSize;
  }
  if (tableState.page > 0) {
    state.page = tableState.page;
  }
  const selectedEntityId = tableState.selectedEntityId ?? tableState.selected?.id;
  if (selectedEntityId) {
    state.selectedEntityId = selectedEntityId;
  }
  return state;
}

export function ViewStateBoundary({
  children,
  engineId,
  queryId,
  activeTab,
  initialPlanId,
  durationSeconds,
  defaultRootResourceType = null,
  operators,
  encodedState,
  maxEncodedLength,
  isQueryReady,
}: ViewStateBoundaryProps) {
  useHydrateSelectedPlanId(initialPlanId);
  const store = useStore();
  const readZoomRange = useReadZoomRange();
  const setZoomRange = useSetZoomRange();
  const setDebouncedZoomRange = useSetDebouncedZoomRange();
  const { read: readSerializableViewState, hydrate: hydrateSerializableViewState } =
    useSerializableViewState({
      operatorTablePersistKey: OPERATOR_TABLE_PERSIST_KEY,
      operatorTableGroupKeys: OPERATOR_TABLE_INDEX_ORDER,
      operators,
    });
  const intakeRoute = useRef({ engineId, queryId, activeTab }).current;
  const [isHydrated, setIsHydrated] = useState(!encodedState);

  const intake = useMemo((): IntakeState => {
    if (!encodedState) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        fields: null,
        isResolved: true,
        status: { kind: 'idle' } satisfies ViewStateIntakeStatus,
      };
    }
    if (!isQueryReady) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        fields: null,
        isResolved: false,
        status: { kind: 'idle' } satisfies ViewStateIntakeStatus,
      };
    }

    const decoded = decodeViewState(encodedState, { maxEncodedLength });
    if (!decoded.ok) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        fields: null,
        isResolved: true,
        status: {
          kind: 'error',
          message: decoded.message,
          messages: decoded.messages,
        } satisfies ViewStateIntakeStatus,
      };
    }

    const fields = readViewStateFields(decoded.value);
    // intakeRoute is captured once via useRef(...).current at mount and never
    // reassigned, so reading it during render carries no tearing risk.
    /* eslint-disable react-hooks/refs */
    if (
      fields.route &&
      (fields.route.engineId !== intakeRoute.engineId ||
        fields.route.queryId !== intakeRoute.queryId ||
        fields.route.tab !== intakeRoute.activeTab)
    ) {
      /* eslint-enable react-hooks/refs */
      const message = 'The saved state does not match the current engine, query, and tab.';
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        fields: null,
        isResolved: true,
        status: {
          kind: 'error',
          message,
          messages: [message],
        },
      };
    }

    const expandedResourceIds = fields.expandedResourceIds ?? null;
    const normalized = fields.zoomRange
      ? normalizeZoomRange(fields.zoomRange, durationSeconds)
      : null;
    if (fields.zoomRange && !normalized) {
      const message = 'The query duration cannot support the restored timeline viewport.';
      return {
        initialExpandedResourceIds: expandedResourceIds,
        initialZoomRange: null,
        fields,
        isResolved: true,
        status: {
          kind: 'error',
          message,
          messages: [message],
        } satisfies ViewStateIntakeStatus,
      };
    }

    return {
      initialExpandedResourceIds: expandedResourceIds,
      initialZoomRange: normalized?.range ?? null,
      fields,
      isResolved: true,
      status: normalized?.wasAdjusted
        ? ({
            kind: 'warning',
            message: 'The restored timeline viewport was adjusted to fit this query.',
          } satisfies ViewStateIntakeStatus)
        : ({ kind: 'ready' } satisfies ViewStateIntakeStatus),
    };
  }, [durationSeconds, encodedState, intakeRoute, isQueryReady, maxEncodedLength]);

  useLayoutEffect(() => {
    if (!intake.isResolved) {
      return;
    }
    if (intake.initialZoomRange) {
      setZoomRange(intake.initialZoomRange);
      setDebouncedZoomRange(intake.initialZoomRange);
    }
    if (intake.initialExpandedResourceIds !== null) {
      store.set(expandedIdsAtom, new Set(intake.initialExpandedResourceIds));
    }
    if (intake.fields) {
      // Restored state describes the whole view, so omitted pins clear session state.
      store.set(pinnedIdsAtom, new Set(intake.fields.resources?.pinnedRowIds ?? []));
      const resourceFilter = intake.fields.resources?.resourceFilter;
      store.set(
        resourceFilterAtom,
        resourceFilter
          ? {
              fsmTypes: resourceFilter.fsmTypes ?? [],
              resourceTypes: resourceFilter.resourceTypes ?? [],
              search: resourceFilter.search ?? '',
              showOthers: resourceFilter.showOthers ?? false,
            }
          : EMPTY_RESOURCE_FILTER
      );
    }
    if (intake.fields) {
      // Like pins, a link without the flag means "hidden", so it overrides the session's setting.
      // expandedIdsAtom is left exactly as restored above.
      store.set(showNestedChartRowsAtom, intake.fields.resources?.showNestedChartRows ?? false);
    }
    if (intake.fields) {
      store.set(sharedCollapsedIdsAtom, new Set(intake.fields.resources?.collapsedRowIds ?? []));
    }
    if (intake.fields?.resources) {
      const resources = intake.fields.resources;
      if (resources.rootResourceType !== undefined) {
        store.set(rootResourceTypeAtom, resources.rootResourceType);
      }
      if (resources.resourceTypeSelections !== undefined) {
        store.set(
          selectedTypesAtom,
          new Map(resources.resourceTypeSelections.map(entry => [entry.rowId, entry.resourceType]))
        );
      }
      if (resources.fsmSelections !== undefined) {
        store.set(
          selectedFsmTypesAtom,
          new Map(resources.fsmSelections.map(entry => [entry.rowId, entry.fsmType]))
        );
      }
    }
    if (intake.fields) {
      const resources = intake.fields.resources;
      store.set(pinnedCollapsedAtom, resources?.pinnedRowsCollapsed ?? false);
      store.set(pinnedPanelSizeAtom, resources?.pinnedRowsPanelSize);
      store.set(selectedNvtxDomainAtom, resources?.nvtxSelection?.domainId ?? null);
      store.set(
        selectedNvtxCategoriesAtom,
        new Map(
          resources?.nvtxSelection?.categories?.map(({ domainId, categoryId }) => [
            domainId,
            categoryId,
          ]) ?? []
        )
      );
      hydrateSerializableViewState({
        selection: intake.fields.selection,
        dag: intake.fields.dag,
        dataFlow: intake.fields.dataFlow
          ? {
              enabled: intake.fields.dataFlow.enabled,
              measure: intake.fields.dataFlow.measure,
              labelMeasure: intake.fields.dataFlow.labelMeasure,
              dimensions: intake.fields.dataFlow.dimensions,
              playheadS: intake.fields.dataFlow.playheadS,
            }
          : undefined,
        operatorTable: intake.fields.operatorTable,
        timeline: {
          longEntityDensity:
            intake.fields.timeline?.longEntityDensity ?? LONG_ENTITY_DENSITY_DEFAULT,
        },
      });
      if (intake.fields.entities) {
        store.set(
          entitiesTableStateAtom,
          hydrateEntitiesState(intake.fields.entities, durationSeconds)
        );
      }
    }
    // eslint-disable-next-line react-hooks/set-state-in-effect -- one-time hydration-complete flag; cannot be derived during render
    setIsHydrated(true);
  }, [
    durationSeconds,
    encodedState,
    intake.initialExpandedResourceIds,
    intake.initialZoomRange,
    intake.isResolved,
    intake.fields,
    hydrateSerializableViewState,
    setDebouncedZoomRange,
    setZoomRange,
    store,
  ]);

  const captureState = useCallback((): CaptureStateResult => {
    if (!queryId || !activeTab) {
      return captureFailure('Select a query tab before capturing view state.');
    }
    const capturedRange =
      activeTab === 'entities' ? null : resolveCapturedZoomRange(readZoomRange(), durationSeconds);
    if (!capturedRange && activeTab !== 'entities') {
      return captureFailure('The timeline viewport is not available yet.');
    }
    const sharedView = readSerializableViewState();
    const entitiesState =
      activeTab === 'entities'
        ? captureEntitiesState(store.get(entitiesTableStateAtom), durationSeconds)
        : undefined;
    if (activeTab === 'entities' && entitiesState === null) {
      return captureFailure('Fix the entity filters before capturing view state.');
    }

    const state: ViewStateV3 = {
      route: { engineId, queryId, tab: activeTab },
    };
    const timeline: NonNullable<ViewStateV3['timeline']> = {};
    if (capturedRange) {
      timeline.zoomRange = capturedRange;
    }
    if (sharedView.timeline.longEntityDensity !== LONG_ENTITY_DENSITY_DEFAULT) {
      timeline.longEntityDensity = sharedView.timeline.longEntityDensity;
    }
    if (hasKeys(timeline)) {
      state.timeline = timeline;
    }

    const selection: NonNullable<ViewStateV3['selection']> = {};
    if (sharedView.selection.planId) {
      selection.planId = sharedView.selection.planId;
    }
    if (sharedView.selection.pipes?.length) {
      selection.pipes = [...sharedView.selection.pipes];
    }
    if (sharedView.selection.operatorNodeIds.length > 0) {
      selection.operatorNodeIds = sharedView.selection.operatorNodeIds;
    }
    if (hasKeys(selection)) {
      state.selection = selection;
    }

    const resources: NonNullable<ViewStateV3['resources']> = {};
    // Rows the app opens by itself are left out. Only the user's changes to them are shared.
    const expandedIds = store.get(expandedIdsAtom);
    const defaultExpandedIds = store.get(defaultExpandedIdsAtom);
    const expandedRowIds = [...expandedIds].filter(id => !defaultExpandedIds.has(id)).sort();
    if (expandedRowIds.length > 0) {
      resources.expandedRowIds = expandedRowIds;
    }
    const collapsedRowIds = [...defaultExpandedIds].filter(id => !expandedIds.has(id)).sort();
    if (collapsedRowIds.length > 0) {
      resources.collapsedRowIds = collapsedRowIds;
    }
    if (store.get(showNestedChartRowsAtom)) {
      resources.showNestedChartRows = true;
    }
    // Keep the order the user arranged the pins in.
    const pinnedRowIds = [...store.get(pinnedIdsAtom)];
    if (pinnedRowIds.length > 0) {
      resources.pinnedRowIds = pinnedRowIds;
    }
    if (store.get(pinnedCollapsedAtom)) {
      resources.pinnedRowsCollapsed = true;
    }
    const pinnedRowsPanelSize = store.get(pinnedPanelSizeAtom);
    if (pinnedRowsPanelSize !== undefined && pinnedRowsPanelSize > 0) {
      resources.pinnedRowsPanelSize = Math.round(pinnedRowsPanelSize);
    }
    const nvtxDomainId = store.get(selectedNvtxDomainAtom);
    const nvtxCategories = [...store.get(selectedNvtxCategoriesAtom)]
      .map(([domainId, categoryId]) => ({ domainId, categoryId }))
      .sort((a, b) => a.domainId.localeCompare(b.domainId));
    if (nvtxDomainId || nvtxCategories.length > 0) {
      resources.nvtxSelection = {
        ...(nvtxDomainId ? { domainId: nvtxDomainId } : {}),
        ...(nvtxCategories.length > 0 ? { categories: nvtxCategories } : {}),
      };
    }
    const resourceFilter = store.get(resourceFilterAtom);
    if (isResourceFilterActive(resourceFilter)) {
      resources.resourceFilter = {
        ...(resourceFilter.search.trim() ? { search: resourceFilter.search.trim() } : {}),
        ...(resourceFilter.resourceTypes.length > 0
          ? { resourceTypes: [...resourceFilter.resourceTypes].sort() }
          : {}),
        ...(resourceFilter.fsmTypes.length > 0
          ? { fsmTypes: [...resourceFilter.fsmTypes].sort() }
          : {}),
        ...(resourceFilter.showOthers ? { showOthers: true } : {}),
      };
    }
    const rootResourceType = store.get(rootResourceTypeAtom);
    if (rootResourceType && rootResourceType !== defaultRootResourceType) {
      resources.rootResourceType = rootResourceType;
    }
    const resourceTypeSelections = [...store.get(selectedTypesAtom)]
      .map(([rowId, resourceType]) => ({ rowId, resourceType }))
      .sort((a, b) => a.rowId.localeCompare(b.rowId));
    if (resourceTypeSelections.length > 0) {
      resources.resourceTypeSelections = resourceTypeSelections;
    }
    const fsmSelections = [...store.get(selectedFsmTypesAtom)]
      .map(([rowId, fsmType]) => ({ rowId, fsmType }))
      .sort((a, b) => a.rowId.localeCompare(b.rowId));
    if (fsmSelections.length > 0) {
      resources.fsmSelections = fsmSelections;
    }
    if (hasKeys(resources)) {
      state.resources = resources;
    }

    const dag: NonNullable<ViewStateV3['dag']> = {};
    if (sharedView.dag.nodeColorField) {
      dag.nodeColorField = sharedView.dag.nodeColorField;
    }
    if (sharedView.dag.nodeColorPalette !== 'blue') {
      dag.nodeColorPalette = sharedView.dag.nodeColorPalette;
    }
    if (sharedView.dag.edgeWidthField) {
      dag.edgeWidthField = sharedView.dag.edgeWidthField;
    }
    if (sharedView.dag.edgeColorField) {
      dag.edgeColorField = sharedView.dag.edgeColorField;
    }
    if (sharedView.dag.edgeColorPalette !== 'teal') {
      dag.edgeColorPalette = sharedView.dag.edgeColorPalette;
    }
    if (sharedView.dag.nodeLabelField !== NODE_LABEL_FIELD.NAME) {
      dag.nodeLabelField = sharedView.dag.nodeLabelField;
    }
    if (sharedView.dag.layoutDirection !== DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP) {
      dag.layoutDirection = sharedView.dag.layoutDirection;
    }
    if (sharedView.dag.scaleType !== SCALE_TYPE.LOG) {
      dag.scaleType = sharedView.dag.scaleType;
    }
    if (hasKeys(dag)) {
      state.dag = dag;
    }

    const dataFlow: NonNullable<ViewStateV3['dataFlow']> = {};
    if (!sharedView.dataFlow.enabled) {
      dataFlow.enabled = false;
    }
    if (sharedView.dataFlow.measure) {
      dataFlow.measure = sharedView.dataFlow.measure;
    }
    if (sharedView.dataFlow.labelMeasure) {
      dataFlow.labelMeasure = sharedView.dataFlow.labelMeasure;
    }
    if (sharedView.dataFlow.dimensions && sharedView.dataFlow.dimensions.length > 0) {
      dataFlow.dimensions = sharedView.dataFlow.dimensions;
    }
    if (
      sharedView.dataFlow.playheadS !== null &&
      (!capturedRange ||
        Math.abs(sharedView.dataFlow.playheadS - capturedRange.start) > Number.EPSILON)
    ) {
      dataFlow.playheadS = sharedView.dataFlow.playheadS;
    }
    if (hasKeys(dataFlow)) {
      state.dataFlow = dataFlow;
    }

    const table: NonNullable<ViewStateV3['operatorTable']> = {};
    const groupingOrder = sharedView.operatorTable.groupingOrder?.filter(isOperatorGroup);
    if (groupingOrder) {
      table.groupingOrder = groupingOrder;
    }
    const enabledGroups = sharedView.operatorTable.enabledGroups?.filter(isOperatorGroup);
    if (
      enabledGroups &&
      !OPERATOR_TABLE_INDEX_ORDER.every(
        key => enabledGroups.includes(key) === DEFAULT_OPERATOR_TABLE_ENABLED[key]
      )
    ) {
      table.enabledGroups = enabledGroups;
    }
    if (sharedView.operatorTable.visibleStats !== undefined) {
      table.visibleStats = sharedView.operatorTable.visibleStats;
    }
    if (sharedView.operatorTable.aggregation !== undefined) {
      table.aggregation = sharedView.operatorTable.aggregation;
    }
    if (sharedView.operatorTable.sort !== undefined) {
      table.sort = sharedView.operatorTable.sort;
    }
    if (hasKeys(table)) {
      state.operatorTable = table;
    }

    if (entitiesState && hasKeys(entitiesState)) {
      state.entities = entitiesState;
    }

    return { ok: true, value: state };
  }, [
    activeTab,
    defaultRootResourceType,
    durationSeconds,
    engineId,
    queryId,
    readSerializableViewState,
    readZoomRange,
    store,
  ]);

  const captureEncodedState = useCallback(
    (options?: ViewStateCodecOptions): EncodedViewStateResult => {
      const captured = captureState();
      if (!captured.ok) {
        return captured;
      }
      const encoded = encodeViewState(captured.value, options);
      if (!encoded.ok) {
        return {
          ok: false,
          reason: ENCODED_VIEW_STATE_FAILURE.UNSAVEABLE,
          message: encoded.message,
          messages: encoded.messages,
        };
      }
      return { ok: true, value: encoded.value };
    },
    [captureState]
  );

  const value = useMemo<ViewStateContextValue>(
    () => ({
      captureEncodedState,
      initialExpandedResourceIds: intake.initialExpandedResourceIds,
      initialZoomRange: intake.initialZoomRange,
      intakeStatus: intake.status,
    }),
    [captureEncodedState, intake.initialExpandedResourceIds, intake.initialZoomRange, intake.status]
  );

  return (
    <ViewStateContext.Provider value={value}>
      {isHydrated ? children : null}
    </ViewStateContext.Provider>
  );
}
