// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/** A parsed struct keeps its identity even when it contains no fields. */
export interface StatStruct {
  kind: 'struct';
  fields: Statistic[];
}

export interface Statistic {
  key: string;
  value: StatValue;
  quantity?: string;
}

export type StatValue = string | number | bigint | boolean | null | StatStruct | StatValue[];

export function isStatStruct(value: StatValue): value is StatStruct {
  return (
    typeof value === 'object' && value !== null && !Array.isArray(value) && value.kind === 'struct'
  );
}
