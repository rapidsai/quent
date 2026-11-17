// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useDefaultLayout, type LayoutStorage } from 'react-resizable-panels';

const PANEL_LAYOUT_PREFIX = 'quent.panelLayout.v1';

// Blocked or full storage just means sizes are not remembered.
const safeStorage: LayoutStorage = {
  getItem(key) {
    try {
      return window.localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  setItem(key, value) {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      // Not remembering the layout is fine.
    }
  },
};

/**
 * Remembers one panel group's sizes across reloads, for every query. Give each
 * group a unique id and spread the result onto the `ResizablePanelGroup`. Every
 * panel in the group needs an `id`, since layouts are saved by panel id.
 */
export function usePanelLayout(groupId: string, panelIds: string[]) {
  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: `${PANEL_LAYOUT_PREFIX}:${groupId}`,
    panelIds,
    storage: safeStorage,
    onlySaveAfterUserInteractions: true,
  });
  return { defaultLayout, onLayoutChanged };
}
