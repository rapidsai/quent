// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState } from 'react';
import { browserStorage, readViewAutosave } from './viewAutosave.storage';
import type { ViewAutosaveRecord } from './viewAutosave.schema';

interface UseRestoredViewOptions {
  engineId: string;
  queryId: string | undefined;
  activeTab: ViewAutosaveRecord['tab'] | undefined;
  /** A shared link in the URL takes priority over the remembered view. */
  hasUrlState: boolean;
  /** Restoring waits until the query and tab are known. */
  isReady: boolean;
}

type Resolved = { scopeKey: string; view: ViewAutosaveRecord | null };

/**
 * Looks up the remembered view once each time a query is opened. The answer is
 * frozen for that visit so later autosaves never re-trigger a restore, and a
 * view saved on a different tab is ignored.
 */
export function useRestoredView({
  engineId,
  queryId,
  activeTab,
  hasUrlState,
  isReady,
}: UseRestoredViewOptions): ViewAutosaveRecord | null {
  const scopeKey = queryId === undefined ? null : `${engineId}:${queryId}`;
  const [resolved, setResolved] = useState<Resolved | null>(null);

  if (scopeKey !== null && queryId !== undefined && isReady && activeTab !== undefined) {
    if (resolved?.scopeKey !== scopeKey) {
      const saved = hasUrlState ? null : readViewAutosave(browserStorage(), { engineId, queryId });
      setResolved({ scopeKey, view: saved?.tab === activeTab ? saved : null });
    }
  }

  return resolved !== null && resolved.scopeKey === scopeKey ? resolved.view : null;
}
