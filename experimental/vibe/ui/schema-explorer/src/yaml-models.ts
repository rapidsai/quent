// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import simple from './models/simple.yaml?raw';
import hello from './models/hello.yaml?raw';
import dynamoInference from './models/dynamo-inference.yaml?raw';
import simulator from './models/simulator.yaml?raw';
import sirius from './models/sirius.yaml?raw';

export type ExampleModelId =
  | 'simple'
  | 'hello'
  | 'dynamo-inference'
  | 'simulator'
  | 'sirius';

export interface YamlExampleModel {
  id: ExampleModelId;
  label: string;
  description: string;
  source: string;
}

export const yamlExampleModels: YamlExampleModel[] = [
  {
    id: 'simple',
    label: 'Simple async executor',
    description: 'One executor, one worker-thread resource, and one task lifecycle.',
    source: simple,
  },
  {
    id: 'hello',
    label: 'Hello world',
    description: 'One application entity with one event.',
    source: hello,
  },
  {
    id: 'dynamo-inference',
    label: 'Dynamo inference deployment',
    description: 'Disaggregated serving, KV-aware routing, tiered cache movement, rollout, and independent pool scaling.',
    source: dynamoInference,
  },
  {
    id: 'simulator',
    label: 'Simulator',
    description: 'Task execution over processor, memory, storage, and network resources.',
    source: simulator,
  },
  {
    id: 'sirius',
    label: 'Sirius',
    description: 'GPU topology, memory tiers, tasks, data batches, and batch placements.',
    source: sirius,
  },
];
