// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, type Dispatch, type SetStateAction, useContext } from 'react';

export type DeepLinkNavTargetContextValue = {
  target: HTMLElement | null;
  setTarget: Dispatch<SetStateAction<HTMLElement | null>>;
};

export const DeepLinkNavTargetContext = createContext<DeepLinkNavTargetContextValue | null>(null);

export function useDeepLinkNavTarget() {
  const context = useContext(DeepLinkNavTargetContext);
  if (!context) {
    throw new Error('DeepLinkNavTargetProvider is missing');
  }
  return context;
}
