// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { defineWorkspace } from 'vitest/config';

export default defineWorkspace([
  // App tests -- existing config unchanged
  './vitest.config.ts',
  // Per-package configs (picked up when created in later phases)
  './packages/@quent/*/vitest.config.ts',
]);
