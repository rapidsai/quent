// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createContext, useContext, useMemo, useState, type ReactNode } from 'react';

interface DAGPreviewContextValue {
  activePreviewId: string | null;
  setActivePreviewId: (id: string | null) => void;
}

const DAGPreviewContext = createContext<DAGPreviewContextValue>({
  activePreviewId: null,
  setActivePreviewId: () => undefined,
});

export function DAGPreviewProvider({ children }: { children: ReactNode }) {
  const [activePreviewId, setActivePreviewId] = useState<string | null>(null);
  const value = useMemo(() => ({ activePreviewId, setActivePreviewId }), [activePreviewId]);
  return <DAGPreviewContext.Provider value={value}>{children}</DAGPreviewContext.Provider>;
}

export const useDAGPreview = () => useContext(DAGPreviewContext);
