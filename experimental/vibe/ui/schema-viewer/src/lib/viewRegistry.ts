// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import GraphView from '../components/GraphView.svelte';
import ResourceTimeline from '../components/ResourceTimeline.svelte';
import { ENTITY_GRAPH_VIEWS } from './types';

export const ENTITY_GRAPH_VIEW_REGISTRY = [
  {
    ...ENTITY_GRAPH_VIEWS[0],
    component: GraphView,
  },
  {
    ...ENTITY_GRAPH_VIEWS[1],
    component: ResourceTimeline,
  },
] as const;
