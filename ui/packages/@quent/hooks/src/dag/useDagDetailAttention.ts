// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValue, useSetAtom } from 'jotai';
import { clearDagDetailAttentionAtom, dagDetailAttentionAtom } from '../atoms/dagDetailAttention';

export const useDagDetailAttention = () => useAtomValue(dagDetailAttentionAtom);
export const useClearDagDetailAttention = () => useSetAtom(clearDagDetailAttentionAtom);
