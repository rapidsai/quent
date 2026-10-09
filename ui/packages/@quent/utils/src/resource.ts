// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { FsmTypeSelection, ResourceGroupTypeDecl, ResourceTypeDecl } from './types';

/** Return the declared FSM choices available for a resource, in group preference order. */
export function resourceFsmChoices(
  declaration: ResourceTypeDecl | undefined,
  groupDeclaration?: ResourceGroupTypeDecl
): FsmTypeSelection[] {
  const choices = declaration?.display_order ?? [];
  if (!groupDeclaration?.display_order.length) {
    return choices;
  }
  const groupChoices = groupDeclaration.display_order.filter(
    selection =>
      selection === 'All' ||
      choices.some(choice => choice !== 'All' && choice.Type === selection.Type)
  );
  return [
    ...groupChoices,
    ...choices.filter(
      choice =>
        choice !== 'All' &&
        !groupChoices.some(selection => selection !== 'All' && selection.Type === choice.Type)
    ),
  ];
}

/** Resolve the FSM filter, preserving explicit selections including null (all FSMs). */
export function resolveResourceFsmType(
  declaration: ResourceTypeDecl | undefined,
  selection?: string | null,
  groupDeclaration?: ResourceGroupTypeDecl
): string | null {
  if (selection !== undefined) {
    return selection;
  }
  const choices = resourceFsmChoices(declaration, groupDeclaration);
  const fsmChoices = choices.filter(choice => choice !== 'All');
  // With a single FSM, "All" is just that FSM's aggregate; show the FSM itself.
  if (fsmChoices.length === 1) {
    return fsmChoices[0]!.Type;
  }
  const initial = choices[0];
  return initial && initial !== 'All' ? initial.Type : null;
}
