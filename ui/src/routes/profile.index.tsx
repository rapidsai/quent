// SPDX-FileCopyrightText: Copyright (c) 2025 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { EngineSelectionPage } from '@/pages/EngineSelectionPage';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/profile/')({
  component: EngineSelectionPage,
});
