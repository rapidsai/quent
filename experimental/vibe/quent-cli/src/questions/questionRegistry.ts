// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { operatorFactsQuestion } from './operatorFacts';
import { operatorStatisticsSummaryQuestion } from './operatorStatisticsSummary';
import { registerQuestion, type RegisteredQuestion } from './question.types';
import { queryLifecycleQuestion } from './queryLifecycle';
import { queryStructureQuestion } from './queryStructure';

const questions: RegisteredQuestion[] = [
  registerQuestion(queryLifecycleQuestion),
  registerQuestion(queryStructureQuestion),
  registerQuestion(operatorFactsQuestion),
  registerQuestion(operatorStatisticsSummaryQuestion),
];

export const questionRegistry: ReadonlyMap<string, RegisteredQuestion> = new Map(
  questions.map(question => [question.metadata.id, question])
);

export function getQuestion(questionId: string): RegisteredQuestion {
  const question = questionRegistry.get(questionId);
  if (!question) {
    const supported = [...questionRegistry.keys()].join(', ');
    throw new Error(`Unknown question "${questionId}". Supported questions: ${supported}.`);
  }
  return question;
}
