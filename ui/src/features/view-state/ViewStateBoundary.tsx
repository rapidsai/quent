// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useStore } from 'jotai';
import {
  useHydrateSelectedPlanId,
  useReadZoomRange,
  useSerializableViewState,
  useSetDebouncedZoomRange,
  useSetZoomRange,
} from '@quent/hooks';
import type { Operator } from '@quent/utils';
import { entitiesTableStateAtom } from '@/atoms/entitiesTable';
import { expandedIdsAtom } from '@/atoms/resourceTree';
import {
  OPERATOR_TABLE_INDEX_ORDER,
  OPERATOR_TABLE_PERSIST_KEY,
} from '@/components/operator-table/types';
import { decodeViewState, encodeViewState, type ViewStateCodecOptions } from './viewState.codec';
import {
  ENCODED_VIEW_STATE_FAILURE,
  ViewStateContext,
  type ViewStateContextValue,
  type ViewStateIntakeStatus,
  type EncodedViewStateResult,
} from './viewState.context';
import {
  captureDag,
  captureDataFlow,
  captureEntitiesState,
  captureOperatorTable,
  captureSelection,
  captureTimeline,
  nonEmpty,
} from './viewState.capture';
import { captureResources } from './viewState.capture.resources';
import { readViewStateFields, type ViewStateFields } from './viewState.fields';
import { hydrateEntitiesState, hydrateResources, toHydratableViewState } from './viewState.hydrate';
import { normalizeZoomRange, resolveCapturedZoomRange } from './viewState.normalize';
import type { ViewStateV3, ViewStateTab } from './viewState.schema';

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

function captureFailure(message: string): ViewFailure {
  return {
    ok: false,
    reason: ENCODED_VIEW_STATE_FAILURE.UNAVAILABLE,
    message,
    messages: [message],
  };
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
      hydrateResources(store, intake.fields.resources);
      hydrateSerializableViewState(toHydratableViewState(intake.fields));
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
    const isEntitiesTab = activeTab === 'entities';
    const capturedRange = isEntitiesTab
      ? null
      : resolveCapturedZoomRange(readZoomRange(), durationSeconds);
    if (!capturedRange && !isEntitiesTab) {
      return captureFailure('The timeline viewport is not available yet.');
    }
    const entitiesState = isEntitiesTab
      ? captureEntitiesState(store.get(entitiesTableStateAtom), durationSeconds)
      : undefined;
    if (entitiesState === null) {
      return captureFailure('Fix the entity filters before capturing view state.');
    }

    const sharedView = readSerializableViewState();
    const timeline = captureTimeline(capturedRange, sharedView.timeline);
    const selection = captureSelection(sharedView.selection);
    const resources = captureResources(store, defaultRootResourceType);
    const dag = captureDag(sharedView.dag);
    const dataFlow = captureDataFlow(sharedView.dataFlow, capturedRange);
    const operatorTable = captureOperatorTable(sharedView.operatorTable);
    const entities = entitiesState && nonEmpty(entitiesState);

    return {
      ok: true,
      value: {
        route: { engineId, queryId, tab: activeTab },
        ...(timeline && { timeline }),
        ...(selection && { selection }),
        ...(resources && { resources }),
        ...(dag && { dag }),
        ...(dataFlow && { dataFlow }),
        ...(operatorTable && { operatorTable }),
        ...(entities && { entities }),
      },
    };
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
