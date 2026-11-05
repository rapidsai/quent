// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';

export interface DAGDetailAttentionRequest {
  token: number;
  kind: 'operator' | 'pipe';
  id: string;
  activateTab: boolean;
}

export const dagDetailAttentionAtom = atom<DAGDetailAttentionRequest | null>(null);
const dagDetailAttentionTokenAtom = atom(0);

export const requestDagDetailAttentionAtom = atom(
  null,
  (get, set, request: Omit<DAGDetailAttentionRequest, 'token'>) => {
    const token = get(dagDetailAttentionTokenAtom) + 1;
    set(dagDetailAttentionTokenAtom, token);
    set(dagDetailAttentionAtom, { ...request, token });
  }
);

export const clearDagDetailAttentionAtom = atom(
  null,
  (get, set, request: DAGDetailAttentionRequest) => {
    if (get(dagDetailAttentionAtom)?.token === request.token) {
      set(dagDetailAttentionAtom, null);
    }
  }
);
