// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useMemo, useRef } from 'react';
import { ENCODED_VIEW_STATE_FAILURE, useViewState } from './viewState.context';
import type { ViewStateTab } from './viewState.schema';
import type { EncodedViewStateRepository } from './viewState.storage';

interface ViewStatePersistenceProps {
  repository: EncodedViewStateRepository;
  engineId: string;
  queryId: string;
  tab: ViewStateTab;
  restoring: boolean;
}

export function ViewStatePersistence({
  repository,
  engineId,
  queryId,
  tab,
  restoring,
}: ViewStatePersistenceProps) {
  const viewState = useViewState();
  const scope = useMemo(() => ({ engineId, queryId, tab }), [engineId, queryId, tab]);
  const captureRef = useRef(viewState?.captureEncodedState);
  const lastStoredState = useRef<string | null>(null);

  useEffect(() => {
    captureRef.current = viewState?.captureEncodedState;
  });

  const removeStored = useCallback(() => {
    try {
      void Promise.resolve(repository.remove(scope)).catch(() => undefined);
    } catch {
      // A failed removal only leaves the old state available for the next visit.
    }
  }, [repository, scope]);

  useEffect(() => {
    if (restoring && viewState?.intakeStatus.kind === 'error') {
      removeStored();
    }
  }, [viewState?.intakeStatus, restoring, removeStored]);

  useEffect(() => {
    if (!viewState) {
      return;
    }
    lastStoredState.current = null;
    const flush = () => {
      const captured = captureRef.current?.({ maxEncodedLength: repository.maxEncodedLength });
      if (captured && !captured.ok && captured.reason === ENCODED_VIEW_STATE_FAILURE.UNSAVEABLE) {
        // Leaving the old snapshot would restore a view the user has since changed.
        lastStoredState.current = null;
        removeStored();
        return;
      }
      if (!captured?.ok || captured.value === lastStoredState.current) {
        return;
      }
      lastStoredState.current = captured.value;
      try {
        void Promise.resolve(repository.save(scope, captured.value)).catch(() => {
          if (lastStoredState.current === captured.value) {
            lastStoredState.current = null;
          }
        });
      } catch {
        lastStoredState.current = null;
      }
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        flush();
      }
    };

    document.addEventListener('visibilitychange', onVisibilityChange);
    window.addEventListener('pagehide', flush);
    return () => {
      document.removeEventListener('visibilitychange', onVisibilityChange);
      window.removeEventListener('pagehide', flush);
      flush();
    };
  }, [viewState, repository, scope, removeStored]);

  return null;
}
