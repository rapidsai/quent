// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export { analyzeOperatorFacts } from './operatorFacts';
export type {
  NamedDynamicValue,
  NamedOperatorStatistic,
  OperatorFactsAnalysis,
} from './operatorFacts';
export { summarizeOperatorStatistics } from './operatorStatisticsSummary';
export type {
  ExtremeSource,
  OperatorStatisticsSummary,
  SerializedNumericValue,
  SkippedReason,
  StatisticPath,
  StatisticScope,
} from './operatorStatisticsSummary';
export { ACTIVE_SPAN_METRIC, commonQueryMetrics, diffQueryBundles } from './queryDiff';
export type {
  QueryDiffBundle,
  QueryDiffComparison,
  QueryDiffOptions,
  QueryDiffResult,
  QueryDiffRow,
  QueryDiffScope,
  QueryDiffSummary,
  SerializedQueryDiffValue,
} from './queryDiff';
export { analyzeQueryLifecycle } from './queryLifecycle';
export type { LifecycleOffset, QueryLifecycleAnalysis } from './queryLifecycle';
export { analyzeQueryStructure } from './queryStructure';
export type { QueryStructureAnalysis, StructureEdge, StructurePlanTree } from './queryStructure';
