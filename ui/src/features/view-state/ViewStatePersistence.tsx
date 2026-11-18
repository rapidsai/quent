// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useRef } from 'react';
import { useViewState } from './viewState.context';
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

  useEffect(() => {
    if (restoring && viewState?.intakeStatus.kind === 'error') {
      try {
        void Promise.resolve(repository.remove(scope)).catch(() => undefined);
      } catch {
        // A failed removal only leaves the rejected state available for the next visit.
      }
    }
  }, [viewState?.intakeStatus, repository, restoring, scope]);

  useEffect(() => {
    if (!viewState) {
      return;
    }
    lastStoredState.current = null;
    const flush = () => {
      const captured = captureRef.current?.();
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
  }, [viewState, repository, scope]);

  return null;
}
