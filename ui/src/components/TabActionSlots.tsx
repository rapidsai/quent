// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, useContext } from 'react';

/** Elements on the left and right of the tab bar that tab content can render into. */
export interface TabActionSlots {
  leftSlot: HTMLElement | null;
  rightSlot: HTMLElement | null;
}

/**
 * `null` means no tab bar hosts the slots, so the tab draws its own controls.
 * Slots that are still `null` mean the tab bar hasn't mounted them yet.
 */
export const TabActionSlotsContext = createContext<TabActionSlots | null>(null);

export function useTabActionSlots(): TabActionSlots | null {
  return useContext(TabActionSlotsContext);
}
