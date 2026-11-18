// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useRef } from 'react';
import type { DeepLinkTab } from './deepLink.schema';
import { useDeepLink } from './deepLink.context';
import {
  browserLocalStorage,
  clearDeepLinkFromLocalStorage,
  writeDeepLinkToLocalStorage,
} from './deepLink.localStorage';

interface LocalDeepLinkStorageProps {
  engineId: string;
  queryId: string;
  tab: DeepLinkTab;
  restoring: boolean;
}

export function LocalDeepLinkStorage({
  engineId,
  queryId,
  tab,
  restoring,
}: LocalDeepLinkStorageProps) {
  const deepLink = useDeepLink();
  const scope = useMemo(() => ({ engineId, queryId }), [engineId, queryId]);
  const captureRef = useRef(deepLink?.captureEncodedState);
  const lastStoredState = useRef<string | null>(null);

  useEffect(() => {
    captureRef.current = deepLink?.captureEncodedState;
  });

  useEffect(() => {
    if (restoring && deepLink?.intakeStatus.kind === 'error') {
      clearDeepLinkFromLocalStorage(browserLocalStorage(), scope);
    }
  }, [deepLink?.intakeStatus, restoring, scope]);

  useEffect(() => {
    if (!deepLink) {
      return;
    }
    lastStoredState.current = null;
    const flush = () => {
      const captured = captureRef.current?.();
      if (!captured?.ok || captured.value === lastStoredState.current) {
        return;
      }
      if (writeDeepLinkToLocalStorage(browserLocalStorage(), scope, captured.value)) {
        lastStoredState.current = captured.value;
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
  }, [deepLink, scope, tab]);

  return null;
}
