// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';

export interface DAGDetailAttentionRequest {
  kind: 'operator' | 'pipe';
  id: string;
}

export const dagDetailAttentionAtom = atom<DAGDetailAttentionRequest | null>(null);

export const requestDagDetailAttentionAtom = atom(
  null,
  (_get, set, request: DAGDetailAttentionRequest) => {
    set(dagDetailAttentionAtom, request);
  }
);

export const clearDagDetailAttentionAtom = atom(
  null,
  (get, set, request: DAGDetailAttentionRequest) => {
    if (get(dagDetailAttentionAtom) === request) {
      set(dagDetailAttentionAtom, null);
    }
  }
);
