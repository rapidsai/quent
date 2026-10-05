// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useAtomValueRawSync } from 'jotai';
import { selectedOperatorIdsAtom } from '../atoms/dag';

export const useSelectedOperatorIds = () => useAtomValueRawSync(selectedOperatorIdsAtom);
