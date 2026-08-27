// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { useStore } from 'jotai';
import {
  useReadZoomRange,
  useSerializableViewState,
  useSetDebouncedZoomRange,
  useSetZoomRange,
} from '@quent/hooks';
import { DAG_LAYOUT_DIRECTION, NODE_LABEL_FIELD } from '@quent/utils';
import { toast } from '@quent/components';
import {
  expandedIdsAtom,
  rootResourceTypeAtom,
  selectedFsmTypesAtom,
  selectedTypesAtom,
} from '@/atoms/resourceTree';
import {
  DEFAULT_OPERATOR_TABLE_ENABLED,
  OPERATOR_TABLE_INDEX_ORDER,
  OPERATOR_TABLE_PERSIST_KEY,
} from '@/components/operator-table/types';
import { buildDeepLinkUrl, decodeDeepLinkState, DEEP_LINK_SEARCH_KEY } from './deepLink.codec';
import {
  DeepLinkContext,
  type CopyLinkResult,
  type DeepLinkContextValue,
  type DeepLinkIntakeStatus,
} from './deepLink.context';
import { readDeepLinkFields, type DeepLinkFields } from './deepLink.fields';
import { normalizeZoomRange, resolveCapturedZoomRange } from './deepLink.normalize';
import { OperatorGroupSchema, type DeepLinkStateV2, type DeepLinkTab } from './deepLink.schema';

interface DeepLinkBoundaryProps {
  children: ReactNode;
  engineId: string;
  queryId?: string;
  activeTab?: DeepLinkTab;
  durationSeconds: number;
  defaultRootResourceType?: string | null;
  encodedState?: string;
  isQueryReady: boolean;
}

type IntakeState = {
  initialExpandedResourceIds: readonly string[] | null;
  initialZoomRange: NonNullable<DeepLinkFields['zoomRange']> | null;
  fields: DeepLinkFields | null;
  isResolved: boolean;
  status: DeepLinkIntakeStatus;
};

function hasKeys(value: object): boolean {
  return Object.keys(value).length > 0;
}

function isOperatorGroup(value: string): value is (typeof OperatorGroupSchema.options)[number] {
  return OperatorGroupSchema.safeParse(value).success;
}

export function DeepLinkBoundary({
  children,
  engineId,
  queryId,
  activeTab,
  durationSeconds,
  defaultRootResourceType = null,
  encodedState,
  isQueryReady,
}: DeepLinkBoundaryProps) {
  const store = useStore();
  const readZoomRange = useReadZoomRange();
  const setZoomRange = useSetZoomRange();
  const setDebouncedZoomRange = useSetDebouncedZoomRange();
  const { read: readSerializableViewState, hydrate: hydrateSerializableViewState } =
    useSerializableViewState({
      operatorTablePersistKey: OPERATOR_TABLE_PERSIST_KEY,
      operatorTableGroupKeys: OPERATOR_TABLE_INDEX_ORDER,
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
        status: { kind: 'idle' } satisfies DeepLinkIntakeStatus,
      };
    }
    if (!isQueryReady) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        fields: null,
        isResolved: false,
        status: { kind: 'idle' } satisfies DeepLinkIntakeStatus,
      };
    }

    const decoded = decodeDeepLinkState(encodedState);
    if (!decoded.ok) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        fields: null,
        isResolved: true,
        status: { kind: 'error', message: decoded.message } satisfies DeepLinkIntakeStatus,
      };
    }

    const fields = readDeepLinkFields(decoded.value);
    if (
      fields.route &&
      (fields.route.engineId !== intakeRoute.engineId ||
        fields.route.queryId !== intakeRoute.queryId ||
        fields.route.tab !== intakeRoute.activeTab)
    ) {
      return {
        initialExpandedResourceIds: null,
        initialZoomRange: null,
        fields: null,
        isResolved: true,
        status: {
          kind: 'error',
          message: 'The shared state does not match the engine, query, and tab in this URL.',
        },
      };
    }

    const expandedResourceIds = fields.expandedResourceIds ?? null;
    const normalized = fields.zoomRange
      ? normalizeZoomRange(fields.zoomRange, durationSeconds)
      : null;
    if (fields.zoomRange && !normalized) {
      return {
        initialExpandedResourceIds: expandedResourceIds,
        initialZoomRange: null,
        fields,
        isResolved: true,
        status: {
          kind: 'error',
          message: 'The query duration cannot support the shared timeline viewport.',
        } satisfies DeepLinkIntakeStatus,
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
            message: 'The shared timeline viewport was adjusted to fit this query.',
          } satisfies DeepLinkIntakeStatus)
        : ({ kind: 'ready' } satisfies DeepLinkIntakeStatus),
    };
  }, [durationSeconds, encodedState, intakeRoute, isQueryReady]);

  useEffect(() => {
    if (intake.status.kind !== 'error' && intake.status.kind !== 'warning') {
      return;
    }

    let cancelled = false;
    queueMicrotask(() => {
      if (cancelled) {
        return;
      }
      if (intake.status.kind === 'error') {
        toast.add({
          id: 'deep-link-intake',
          type: 'error',
          title: 'Could not restore shared view',
          description: 'Some shared view settings were invalid and could not be restored.',
          priority: 'high',
        });
      } else if (intake.status.kind === 'warning') {
        toast.add({
          id: 'deep-link-intake',
          type: 'warning',
          title: 'Shared view adjusted',
          description: intake.status.message,
        });
      }
    });

    return () => {
      cancelled = true;
    };
  }, [intake.status]);

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
      });
    }
    if (encodedState) {
      const url = new URL(window.location.href);
      url.searchParams.delete(DEEP_LINK_SEARCH_KEY);
      window.history.replaceState(
        window.history.state,
        '',
        `${url.pathname}${url.search}${url.hash}`
      );
    }
    setIsHydrated(true);
  }, [
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

  const copyLink = useCallback(async (): Promise<CopyLinkResult> => {
    if (!queryId || !activeTab) {
      return { ok: false, message: 'Select a query tab before copying a shared link.' };
    }
    const capturedRange = resolveCapturedZoomRange(readZoomRange(), durationSeconds);
    if (!capturedRange) {
      return { ok: false, message: 'The timeline viewport is not available yet.' };
    }

    const sharedView = readSerializableViewState();
    const state: DeepLinkStateV2 = {
      route: { engineId, queryId, tab: activeTab },
      timeline: { zoomRange: capturedRange },
    };

    const selection: NonNullable<DeepLinkStateV2['selection']> = {};
    if (sharedView.selection.planId) {
      selection.planId = sharedView.selection.planId;
    }
    if (sharedView.selection.operatorNodeIds.length > 0) {
      selection.operatorNodeIds = sharedView.selection.operatorNodeIds;
    }
    if (hasKeys(selection)) {
      state.selection = selection;
    }

    const resources: NonNullable<DeepLinkStateV2['resources']> = {};
    const expandedRowIds = [...store.get(expandedIdsAtom)].sort();
    if (expandedRowIds.length > 0) {
      resources.expandedRowIds = expandedRowIds;
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

    const dag: NonNullable<DeepLinkStateV2['dag']> = {};
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
    if (hasKeys(dag)) {
      state.dag = dag;
    }

    const dataFlow: NonNullable<DeepLinkStateV2['dataFlow']> = {};
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
      Math.abs(sharedView.dataFlow.playheadS - capturedRange.start) > Number.EPSILON
    ) {
      dataFlow.playheadS = sharedView.dataFlow.playheadS;
    }
    if (hasKeys(dataFlow)) {
      state.dataFlow = dataFlow;
    }

    const table: NonNullable<DeepLinkStateV2['operatorTable']> = {};
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

    const canonicalPageUrl = `${window.location.origin}${window.location.pathname}`;
    const result = buildDeepLinkUrl(canonicalPageUrl, state);
    if (!result.ok) {
      return { ok: false, message: result.message };
    }
    if (!navigator.clipboard?.writeText) {
      return { ok: false, message: 'Clipboard access is unavailable.' };
    }

    try {
      await navigator.clipboard.writeText(result.value);
      return { ok: true, url: result.value };
    } catch {
      return { ok: false, message: 'Could not copy the link to the clipboard.' };
    }
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

  const value = useMemo<DeepLinkContextValue>(
    () => ({
      copyLink,
      initialExpandedResourceIds: intake.initialExpandedResourceIds,
      initialZoomRange: intake.initialZoomRange,
      intakeStatus: intake.status,
    }),
    [copyLink, intake.initialExpandedResourceIds, intake.initialZoomRange, intake.status]
  );

  return (
    <DeepLinkContext.Provider value={value}>
      {isHydrated ? children : null}
    </DeepLinkContext.Provider>
  );
}
