// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState } from 'react';
import { decodeDeepLinkState } from './deepLink.codec';
import { readDeepLinkFields } from './deepLink.fields';
import {
  browserLocalStorage,
  clearDeepLinkFromLocalStorage,
  readDeepLinkFromLocalStorage,
} from './deepLink.localStorage';
import type { DeepLinkTab } from './deepLink.schema';

interface UseLocalDeepLinkStateOptions {
  engineId: string;
  queryId: string | undefined;
  activeTab: DeepLinkTab | undefined;
  hasUrlState: boolean;
  isReady: boolean;
}

type Resolved = { scopeKey: string; encodedState: string | null };

export function useLocalDeepLinkState({
  engineId,
  queryId,
  activeTab,
  hasUrlState,
  isReady,
}: UseLocalDeepLinkStateOptions): string | null {
  const scopeKey = queryId === undefined ? null : `${engineId}:${queryId}`;
  const [resolved, setResolved] = useState<Resolved | null>(null);

  if (
    scopeKey !== null &&
    queryId !== undefined &&
    activeTab !== undefined &&
    isReady &&
    resolved?.scopeKey !== scopeKey
  ) {
    const scope = { engineId, queryId };
    const stored = hasUrlState ? null : readDeepLinkFromLocalStorage(browserLocalStorage(), scope);
    const decoded = stored ? decodeDeepLinkState(stored) : null;
    const route = decoded?.ok ? readDeepLinkFields(decoded.value).route : undefined;
    if (
      stored &&
      (!decoded?.ok || !route || route.engineId !== engineId || route.queryId !== queryId)
    ) {
      clearDeepLinkFromLocalStorage(browserLocalStorage(), scope);
    }
    const encodedState =
      stored && route?.engineId === engineId && route.queryId === queryId && route.tab === activeTab
        ? stored
        : null;
    setResolved({ scopeKey, encodedState });
  }

  return resolved?.scopeKey === scopeKey ? resolved.encodedState : null;
}
