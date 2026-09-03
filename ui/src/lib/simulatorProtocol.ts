// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  BulkTimelineRequest,
  CategoricalTimelineRequest,
  EntityListRequest,
  OperatorFilter,
  QueryFilter,
  SingleTimelineRequest,
} from '@quent/utils';

export type SimulatorOperation =
  | { operation: 'listEngines' }
  | { operation: 'engineContexts'; engineId: string }
  | { operation: 'listCoordinators'; engineId: string }
  | { operation: 'listQueries'; engineId: string; queryGroupId: string }
  | { operation: 'queryBundle'; engineId: string; queryId: string }
  | {
      operation: 'singleTimeline';
      engineId: string;
      request: SingleTimelineRequest<QueryFilter, OperatorFilter>;
    }
  | {
      operation: 'bulkTimelines';
      engineId: string;
      request: BulkTimelineRequest<QueryFilter, OperatorFilter>;
    }
  | {
      operation: 'dataFlow';
      engineId: string;
      request: CategoricalTimelineRequest<QueryFilter>;
    }
  | {
      operation: 'entityList';
      engineId: string;
      request: EntityListRequest<QueryFilter, OperatorFilter>;
    };

export type SimulatorWorkerMessage =
  { type: 'init'; dataUrl: string } | ({ type: 'operation' } & SimulatorOperation);

export type SimulatorWorkerRequest = SimulatorWorkerMessage & { id: number };

export type SimulatorWorkerReply = { id: number; response: string } | { id: number; error: string };
