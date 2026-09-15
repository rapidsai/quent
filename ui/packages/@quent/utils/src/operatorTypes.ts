// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { StatValue } from './dagTypes';
import type { Operator, Plan, Worker } from './types';

export interface OperatorSelection {
  readonly label: string;
  readonly operatorIds: ReadonlySet<string>;
}

export interface OperatorSelectionInput extends OperatorSelection {
  readonly selectionId: string;
}

export interface OperatorSelectionState {
  readonly selections: ReadonlyMap<string, OperatorSelection>;
}

export interface SelectedOperatorData {
  nodeId: string;
  label: string;
  operationType: string;
  statistics: Array<{ key: string; value: StatValue; quantity?: string }>;
  workerLabel?: string;
}

export interface SelectedOperatorGroupData extends SelectedOperatorData {
  relatedOperators?: SelectedOperatorData[];
}

function resolveOperatorWorker(
  operator: Operator,
  plans: Record<string, Plan>,
  workers: Record<string, Worker>
): { plan?: Plan; worker?: Worker } {
  const plan = operator.plan_id ? plans[operator.plan_id] : undefined;
  const worker = plan?.worker_id ? workers[plan.worker_id] : undefined;
  return { plan, worker };
}

export function operatorLocationDescription(
  operator: Operator,
  plans: Record<string, Plan>,
  workers: Record<string, Worker>
): string | undefined {
  const { plan, worker } = resolveOperatorWorker(operator, plans, workers);
  if (!plan) {
    return undefined;
  }
  const planLabel = plan.instance_name ?? plan.id;
  const workerLabel = worker ? (worker.instance_name ?? worker.id) : undefined;
  return workerLabel ? `Plan: ${planLabel} · Worker: ${workerLabel}` : `Plan: ${planLabel}`;
}

export function operatorWorkerLabel(
  operator: Operator,
  plans: Record<string, Plan>,
  workers: Record<string, Worker>
): string | undefined {
  const { worker } = resolveOperatorWorker(operator, plans, workers);
  return worker ? (worker.instance_name ?? worker.id) : undefined;
}
