// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { buildEvidenceDeepLink, type QueryEvidenceDeepLinkState } from '../deepLink';
import type { QuestionContext, QuestionResult } from './question.types';

export function validateQueryBundleContext(context: QuestionContext): void {
  if (context.queryBundle.query_id !== context.queryId) {
    throw new Error(
      `Query bundle ID "${context.queryBundle.query_id}" does not match requested query "${context.queryId}".`
    );
  }
}

export function queryBundleResolution(): QuestionResult['resolution'] {
  return {
    source: 'query-bundle',
    binCount: null,
    binDurationSeconds: null,
    explanation: 'Values are projected from one query bundle; no timeline bins are used.',
  };
}

export function buildQueryBundleDeepLink(
  context: QuestionContext,
  options: {
    tab: QueryEvidenceDeepLinkState['route']['tab'];
    planId?: string;
    operatorIds?: string[];
  }
): string {
  const state: QueryEvidenceDeepLinkState = {
    route: {
      engineId: context.engineId,
      queryId: context.queryId,
      tab: options.tab,
    },
    timeline: {
      zoomRange: {
        start: 0,
        end: context.queryBundle.duration_s,
      },
    },
    ...(options.planId || options.operatorIds?.length
      ? {
          selection: {
            ...(options.planId ? { planId: options.planId } : {}),
            ...(options.operatorIds?.length ? { operatorNodeIds: options.operatorIds } : {}),
          },
        }
      : {}),
  };
  const route = `/profile/engine/${encodeURIComponent(
    context.engineId
  )}/query/${encodeURIComponent(context.queryId)}/${options.tab}`;
  const currentUrl = context.appBaseUrl ? new URL(route, context.appBaseUrl).toString() : route;
  return buildEvidenceDeepLink(currentUrl, state);
}

export function formatSeconds(value: number): string {
  return `${value.toFixed(9).replace(/\.?0+$/u, '')}s`;
}
