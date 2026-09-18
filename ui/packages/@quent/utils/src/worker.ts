// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Worker } from './types';

export function workerDisplayName(worker: Worker): string {
  return worker.instance_name ?? worker.id;
}
