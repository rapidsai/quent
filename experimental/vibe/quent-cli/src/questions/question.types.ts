// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  EntityListRequest,
  EntityListResponse,
  EntityRef,
  OperatorFilter,
  QueryBundle,
  QueryFilter,
} from '@quent/utils';

export type EvidenceClass = 'observed' | 'derived' | 'inferred' | 'unavailable';

export type QuestionCliValues = Record<string, string | boolean | undefined>;

export interface QuestionApi {
  fetchEntityList(
    engineId: string,
    request: EntityListRequest<QueryFilter, OperatorFilter>
  ): Promise<EntityListResponse>;
}

export interface QuestionContext {
  engineId: string;
  queryId: string;
  appBaseUrl?: string;
  queryBundle: QueryBundle<EntityRef>;
  api: QuestionApi;
}

export interface QuestionParameter {
  name: string;
  required: boolean;
  description: string;
}

export interface QuestionMetadata {
  id: string;
  version: number;
  title: string;
  explanation: string;
  requirements: {
    apis: readonly ('query-bundle' | 'entity-list' | 'bulk-timelines' | 'data-flow')[];
    resourceTypes: readonly string[];
    capacities: readonly string[];
    fsmStates: readonly string[];
    topologyFields: readonly string[];
  };
  parameters: readonly QuestionParameter[];
  limitations: readonly string[];
}

export interface QuestionResult {
  question: Pick<QuestionMetadata, 'id' | 'version' | 'title'>;
  evidence: {
    class: EvidenceClass;
    explanation: string;
    confidence?: number;
  };
  resolution: {
    source: 'query-bundle' | 'entity-list' | 'binned-timeline';
    binCount: number | null;
    binDurationSeconds: number | null;
    explanation: string;
  };
  deepLink: string;
  limitations: string[];
}

export interface QuestionDefinition<Input, Result extends QuestionResult> {
  metadata: QuestionMetadata;
  parseInput(values: QuestionCliValues): Input;
  run(context: QuestionContext, input: Input): Promise<Result>;
  formatHuman(result: Result): string;
}

export interface RegisteredQuestion {
  metadata: QuestionMetadata;
  run(context: QuestionContext, values: QuestionCliValues): Promise<QuestionResult>;
  formatHuman(result: QuestionResult): string;
}

export function registerQuestion<Input, Result extends QuestionResult>(
  definition: QuestionDefinition<Input, Result>
): RegisteredQuestion {
  return {
    metadata: definition.metadata,
    run: (context, values) => definition.run(context, definition.parseInput(values)),
    formatHuman: result => definition.formatHuman(result as Result),
  };
}
