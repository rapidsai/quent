// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  Annotations,
  Cardinality,
  DataType,
  Entity,
  Event,
  Field,
  Path,
  Record as SchemaRecord,
  Schema,
} from '@quent/schema';

const REF_TARGET = 'quent.ref-target.v0.1.0';
const REF_TREE = 'quent.ref-tree.v0.1.0';
const FSM = 'quent.fsm.v0.1.0';
const RESOURCE = 'quent.resource.v0.1.0';

type ConstraintEntry = [name: string, data: string | null];
type MetadataEntries = Record<string, string | null>;

function path(value: string): Path {
  const segments = value.split('::');
  return {
    namespace: segments.slice(0, -1),
    name: segments.at(-1)!,
  };
}

function annotations(
  docs: string | null = null,
  constraints: ConstraintEntry[] = [],
  metadata: MetadataEntries = {},
): Annotations {
  return {
    docs,
    constraints: Object.fromEntries(
      constraints.map(([name, data]) => [name, { name, data }]),
    ),
    metadata: Object.fromEntries(
      Object.entries(metadata).map(([name, data]) => [
        name,
        { name, data },
      ]),
    ),
  };
}

function field(
  name: string,
  ty: DataType,
  docs: string | null = null,
  metadata: MetadataEntries = {},
): Field {
  return { name, ty, annotations: annotations(docs, [], metadata) };
}

function event(
  name: string,
  fields: Field[] = [],
  cardinality: Cardinality = 'Once',
  docs: string | null = null,
): Event {
  return {
    name,
    cardinality,
    payload: Object.fromEntries(fields.map((value) => [value.name, value])),
    annotations: annotations(docs),
  };
}

function record(
  name: string,
  fields: Field[],
  annotation: Annotations = annotations(),
): SchemaRecord {
  return {
    path: path(name),
    fields: Object.fromEntries(fields.map((value) => [value.name, value])),
    annotations: annotation,
  };
}

function entity(
  name: string,
  events: Event[],
  annotation: Annotations = annotations(),
): Entity {
  return {
    path: path(name),
    events: Object.fromEntries(events.map((value) => [value.name, value])),
    annotations: annotation,
  };
}

function recordType(name: string): DataType {
  return { Record: path(name) };
}

function optional(type: DataType): DataType {
  return { Option: type };
}

function list(type: DataType): DataType {
  return { List: type };
}

function entityReference(
  target: string | null,
  options: {
    tree?: boolean;
    data?: DataType | null;
    docs?: string;
    metadata?: MetadataEntries;
  } = {},
): DataType {
  const constraints: ConstraintEntry[] = [];
  if (target) {
    constraints.push([REF_TARGET, target]);
  }
  if (options.tree) {
    constraints.push([REF_TREE, null]);
  }
  return {
    EntityRef: {
      data: options.data ?? null,
      annotations: annotations(
        options.docs ?? null,
        constraints,
        options.metadata,
      ),
    },
  };
}

function parentReference(target: string): Field {
  return field(
    'parent',
    entityReference(target, {
      tree: true,
      docs: `Tree parent of this entity.`,
    }),
  );
}

function simpleEntity(
  name: string,
  parent: string,
  options: {
    createdFields?: Field[];
    events?: Event[];
    docs?: string;
    metadata?: MetadataEntries;
    constraints?: ConstraintEntry[];
    parentType?: DataType;
  } = {},
): Entity {
  return entity(
    name,
    [
      event(
        'created',
        [
          field('parent', options.parentType ?? entityReference(parent, { tree: true })),
          ...(options.createdFields ?? []),
        ],
        'Once',
        `Creates a ${name} entity.`,
      ),
      ...(options.events ?? []),
    ],
    annotations(
      options.docs ?? `${name} declaration.`,
      options.constraints ?? [],
      options.metadata,
    ),
  );
}

interface Transition {
  source: string;
  target: string;
}

interface FsmEntityOptions {
  name: string;
  parent: string;
  states: string[];
  initial: string;
  transitions: Transition[];
  exits: string[];
  fields?: Partial<Record<string, Field[]>>;
  docs?: string;
  metadata?: MetadataEntries;
  parentType?: DataType;
}

function fsmEntity(options: FsmEntityOptions): Entity {
  const topology = {
    initial_state: options.initial,
    transitions: options.transitions,
    exit_from_states: {
      state: options.exits[0],
      others: options.exits.slice(1),
    },
  };
  const events = options.states.map((state) => {
    const fields = [...(options.fields?.[state] ?? [])];
    if (state === options.initial) {
      fields.unshift(
        field(
          'parent',
          options.parentType ??
            entityReference(options.parent, {
              tree: true,
              docs: `Tree parent declared by the initial state.`,
            }),
        ),
      );
    }
    return event(
      state,
      fields,
      stateIsCyclic(state, options.transitions) ? 'Multi' : 'Once',
      `${options.name} enters the ${state} state.`,
    );
  });
  return entity(
    options.name,
    events,
    annotations(
      options.docs ?? `${options.name} lifecycle.`,
      [[FSM, JSON.stringify(topology)]],
      options.metadata,
    ),
  );
}

function stateIsCyclic(state: string, transitions: Transition[]): boolean {
  const next = new Map<string, string[]>();
  for (const transition of transitions) {
    const targets = next.get(transition.source) ?? [];
    targets.push(transition.target);
    next.set(transition.source, targets);
  }
  const stack = [...(next.get(state) ?? [])];
  const visited = new Set<string>();
  while (stack.length > 0) {
    const current = stack.pop()!;
    if (current === state) {
      return true;
    }
    if (visited.has(current)) {
      continue;
    }
    visited.add(current);
    stack.push(...(next.get(current) ?? []));
  }
  return false;
}

interface Capacity {
  kind: 'occupancy' | 'rate';
  bounded: boolean;
}

function resourceDefinition(
  name: string,
  parent: string,
  capacities: Record<string, Capacity>,
  boundsRecord: string | null,
): Entity {
  const events = [
    event('created', [
      parentReference(parent),
      field('resource_id', 'Uuid'),
    ]),
  ];
  if (boundsRecord) {
    events.push(
      event(
        'bounds_changed',
        [field('bounds', optional(recordType(boundsRecord)))],
        'Multi',
      ),
    );
  }
  return entity(
    name,
    events,
    annotations(
      `${name} resource definition.`,
      [[RESOURCE, JSON.stringify({ definition: capacities })]],
      { 'quent.example.tier': 'infrastructure' },
    ),
  );
}

function resourceRecord(
  name: string,
  role: 'usage' | 'bounds',
  resource: string,
  fields: Field[],
): SchemaRecord {
  return record(
    name,
    fields,
    annotations(
      `${role} values for ${resource}.`,
      [[RESOURCE, JSON.stringify({ [role]: { resource: path(resource) } })]],
    ),
  );
}

function resourceReference(
  resource: string,
  usageRecord: string,
  options: { optional?: boolean } = {},
): DataType {
  const reference = entityReference(resource, {
    data: recordType(usageRecord),
    docs: `Carries a ${usageRecord} claim.`,
  });
  return options.optional ? optional(reference) : reference;
}

const records: SchemaRecord[] = [
  record(
    'Common::AllScalars',
    [
      field('enabled', 'Bool'),
      field('id', 'Uuid'),
      field('name', 'String'),
      field('u8_value', 'U8'),
      field('u16_value', 'U16'),
      field('u32_value', 'U32'),
      field('u64_value', 'U64'),
      field('i8_value', 'I8'),
      field('i16_value', 'I16'),
      field('i32_value', 'I32'),
      field('i64_value', 'I64'),
      field('f32_value', 'F32'),
      field('f64_value', 'F64'),
    ],
    annotations(
      'Exercises every scalar data type.',
      [['quent.example.record-shape.v0.1.0', '{"shape":"wide"}']],
      { 'quent.example.owner': 'schema-viewer' },
    ),
  ),
  record('Common::Coordinates', [
    field('x', 'F64'),
    field('y', 'F64'),
    field('z', optional('F64')),
  ]),
  record('Common::Envelope', [
    field('id', 'Uuid'),
    field('labels', list('String')),
    field('position', optional(recordType('Common::Coordinates'))),
    field('attributes', 'DynamicRecord'),
  ]),
  record('Common::NestedEnvelope', [
    field('header', recordType('Common::Envelope')),
    field('samples', list(list('F32'))),
    field('scalars', recordType('Common::AllScalars')),
  ]),
  record('Relations::NodeParent', [
    field(
      'node',
      entityReference('Infrastructure::Node', {
        tree: true,
        docs: 'Nested ref-tree edge to a node.',
      }),
    ),
    field('rack_slot', 'U16'),
  ]),
  record('Config::Worker', [
    field('threads', 'U16'),
    field('labels', list('String')),
    field('environment', 'DynamicRecord'),
    field('checkpoint_interval_ms', optional('U64')),
  ]),
  record('Config::Database', [
    field('replicas', 'U8'),
    field('shards', 'U16'),
    field('coordinates', recordType('Common::Coordinates')),
  ]),
  resourceRecord('ResourceData::CpuBounds', 'bounds', 'Resource::Cpu', [
    field('cores', 'U16'),
  ]),
  resourceRecord('ResourceData::CpuUsage', 'usage', 'Resource::Cpu', [
    field('cores', 'U16'),
    field('instructions', 'U64'),
  ]),
  resourceRecord(
    'ResourceData::MemoryBounds',
    'bounds',
    'Resource::Memory',
    [field('bytes', 'U64'), field('bandwidth', 'U64')],
  ),
  resourceRecord(
    'ResourceData::MemoryUsage',
    'usage',
    'Resource::Memory',
    [field('bytes', 'U64'), field('bandwidth', 'U64')],
  ),
  resourceRecord('ResourceData::GpuBounds', 'bounds', 'Resource::Gpu', [
    field('slices', 'U8'),
  ]),
  resourceRecord('ResourceData::GpuUsage', 'usage', 'Resource::Gpu', [
    field('slices', 'U8'),
    field('flops', 'U64'),
  ]),
  resourceRecord('ResourceData::DiskBounds', 'bounds', 'Resource::Disk', [
    field('bytes', 'U64'),
    field('throughput', 'U64'),
  ]),
  resourceRecord('ResourceData::DiskUsage', 'usage', 'Resource::Disk', [
    field('bytes', 'U64'),
    field('throughput', 'U64'),
  ]),
  resourceRecord(
    'ResourceData::NetworkBounds',
    'bounds',
    'Resource::NetworkLink',
    [field('bandwidth', 'U64')],
  ),
  resourceRecord(
    'ResourceData::NetworkUsage',
    'usage',
    'Resource::NetworkLink',
    [field('bandwidth', 'U64')],
  ),
  resourceRecord(
    'ResourceData::SlotUsage',
    'usage',
    'Resource::ExecutionSlot',
    [],
  ),
  record('WorkloadData::ArtifactInfo', [
    field('digest', 'String'),
    field('size_bytes', 'U64'),
    field('replicas', list(entityReference('Workload::Dataset'))),
  ]),
  record('ObservabilityData::SpanContext', [
    field('trace_id', 'Uuid'),
    field('span_id', 'Uuid'),
    field('baggage', 'DynamicRecord'),
  ]),
  record('SecurityData::Claims', [
    field('subject', 'String'),
    field('roles', list('String')),
    field('expires_at', 'I64'),
  ]),
];

const platform = entity(
  'Platform',
  [
    event('bootstrapped', [
      field('version', 'String'),
      field('schema_snapshot', recordType('Common::NestedEnvelope')),
    ]),
    event(
      'configuration_changed',
      [field('configuration', 'DynamicRecord')],
      'Multi',
    ),
  ],
  annotations(
    'Root of the stress-test entity hierarchy.',
    [['quent.example.singleton.v0.1.0', null]],
    {
      'quent.example.environment': 'stress',
      'quent.example.scale': 'large',
    },
  ),
);

const infrastructure = [
  simpleEntity('Infrastructure::Region', 'Platform', {
    createdFields: [
      field('code', 'String'),
      field('coordinates', recordType('Common::Coordinates')),
    ],
    events: [
      event('degraded', [field('reason', 'String')], 'Multi'),
      event('recovered', [], 'Multi'),
    ],
  }),
  simpleEntity('Infrastructure::Cluster', 'Infrastructure::Region', {
    createdFields: [
      field('name', 'String'),
      field('labels', list('String')),
    ],
    events: [
      event(
        'autoscaled',
        [field('previous_size', 'U32'), field('new_size', 'U32')],
        'Multi',
      ),
    ],
  }),
  simpleEntity('Infrastructure::Node', 'Infrastructure::Cluster', {
    createdFields: [
      field('hostname', 'String'),
      field('architecture', 'String'),
      field('inventory', recordType('Common::AllScalars')),
    ],
    events: [
      event('heartbeat', [field('telemetry', 'DynamicRecord')], 'Multi'),
      event('cordoned', [field('reason', optional('String'))], 'Multi'),
    ],
  }),
];

const resources = [
  resourceDefinition(
    'Resource::Cpu',
    'Infrastructure::Node',
    {
      cores: { kind: 'occupancy', bounded: true },
      instructions: { kind: 'rate', bounded: false },
    },
    'ResourceData::CpuBounds',
  ),
  resourceDefinition(
    'Resource::Memory',
    'Infrastructure::Node',
    {
      bytes: { kind: 'occupancy', bounded: true },
      bandwidth: { kind: 'rate', bounded: true },
    },
    'ResourceData::MemoryBounds',
  ),
  resourceDefinition(
    'Resource::Gpu',
    'Infrastructure::Node',
    {
      slices: { kind: 'occupancy', bounded: true },
      flops: { kind: 'rate', bounded: false },
    },
    'ResourceData::GpuBounds',
  ),
  resourceDefinition(
    'Resource::Disk',
    'Infrastructure::Node',
    {
      bytes: { kind: 'occupancy', bounded: true },
      throughput: { kind: 'rate', bounded: true },
    },
    'ResourceData::DiskBounds',
  ),
  resourceDefinition(
    'Resource::NetworkLink',
    'Infrastructure::Cluster',
    {
      bandwidth: { kind: 'rate', bounded: true },
    },
    'ResourceData::NetworkBounds',
  ),
  resourceDefinition(
    'Resource::ExecutionSlot',
    'Infrastructure::Cluster',
    {},
    null,
  ),
];

const workloads = [
  simpleEntity('Workload::Tenant', 'Platform', {
    createdFields: [
      field('tenant_id', 'Uuid'),
      field('display_name', 'String'),
    ],
  }),
  simpleEntity('Workload::Project', 'Workload::Tenant', {
    createdFields: [
      field('project_id', 'Uuid'),
      field('labels', list('String')),
    ],
  }),
  simpleEntity('Workload::Dataset', 'Workload::Project', {
    createdFields: [
      field('uri', 'String'),
      field('schema', 'DynamicRecord'),
    ],
    events: [
      event('discovered', [
        parentReference('Workload::Project'),
        field('source', 'String'),
      ]),
      event(
        'snapshot_created',
        [
          field('snapshot_id', 'Uuid'),
          field(
            'upstream',
            list(entityReference('Workload::Dataset', { data: 'String' })),
          ),
        ],
        'Multi',
      ),
    ],
  }),
  fsmEntity({
    name: 'Workload::Job',
    parent: 'Workload::Project',
    states: ['submitted', 'queued', 'running', 'completed', 'failed'],
    initial: 'submitted',
    transitions: [
      { source: 'submitted', target: 'queued' },
      { source: 'queued', target: 'running' },
      { source: 'running', target: 'queued' },
      { source: 'running', target: 'completed' },
      { source: 'running', target: 'failed' },
    ],
    exits: ['completed', 'failed'],
    fields: {
      submitted: [
        field('job_id', 'Uuid'),
        field('request', recordType('Common::Envelope')),
      ],
      running: [
        field(
          'cpu',
          resourceReference('Resource::Cpu', 'ResourceData::CpuUsage'),
        ),
        field('scheduler', entityReference('Service::Scheduler')),
        field(
          'dataset',
          optional(entityReference('Workload::Dataset', { data: 'String' })),
        ),
      ],
      completed: [field('result', 'DynamicRecord')],
      failed: [field('error', 'String')],
    },
  }),
  fsmEntity({
    name: 'Workload::Task',
    parent: 'Workload::Job',
    states: ['created', 'scheduled', 'executing', 'succeeded', 'failed'],
    initial: 'created',
    transitions: [
      { source: 'created', target: 'scheduled' },
      { source: 'scheduled', target: 'executing' },
      { source: 'executing', target: 'executing' },
      { source: 'executing', target: 'scheduled' },
      { source: 'executing', target: 'succeeded' },
      { source: 'executing', target: 'failed' },
    ],
    exits: ['succeeded', 'failed'],
    fields: {
      created: [field('task_id', 'Uuid')],
      executing: [
        field(
          'cpu',
          resourceReference('Resource::Cpu', 'ResourceData::CpuUsage'),
        ),
        field(
          'memory',
          resourceReference('Resource::Memory', 'ResourceData::MemoryUsage'),
        ),
        field(
          'gpu',
          resourceReference('Resource::Gpu', 'ResourceData::GpuUsage', {
            optional: true,
          }),
        ),
      ],
    },
  }),
  fsmEntity({
    name: 'Workload::Attempt',
    parent: 'Workload::Task',
    states: ['allocated', 'running', 'retrying', 'finished', 'aborted'],
    initial: 'allocated',
    transitions: [
      { source: 'allocated', target: 'running' },
      { source: 'running', target: 'retrying' },
      { source: 'retrying', target: 'running' },
      { source: 'running', target: 'finished' },
      { source: 'running', target: 'aborted' },
    ],
    exits: ['finished', 'aborted'],
    fields: {
      allocated: [
        field(
          'slot',
          resourceReference(
            'Resource::ExecutionSlot',
            'ResourceData::SlotUsage',
          ),
        ),
      ],
      retrying: [field('attempt_number', 'U16')],
    },
  }),
  simpleEntity('Workload::Artifact', 'Workload::Job', {
    createdFields: [
      field('info', recordType('WorkloadData::ArtifactInfo')),
      field('producer', entityReference('Workload::Attempt')),
    ],
    events: [
      event(
        'replicated',
        [field('destinations', list('String'))],
        'Multi',
      ),
    ],
  }),
];

const services = [
  fsmEntity({
    name: 'Service::Scheduler',
    parent: 'Infrastructure::Cluster',
    states: ['starting', 'idle', 'scheduling', 'stopped'],
    initial: 'starting',
    transitions: [
      { source: 'starting', target: 'idle' },
      { source: 'idle', target: 'scheduling' },
      { source: 'scheduling', target: 'idle' },
      { source: 'idle', target: 'stopped' },
    ],
    exits: ['stopped'],
    fields: {
      scheduling: [
        field(
          'cpu',
          resourceReference('Resource::Cpu', 'ResourceData::CpuUsage'),
        ),
        field('job', entityReference('Workload::Job')),
      ],
    },
  }),
  fsmEntity({
    name: 'Service::Worker',
    parent: 'Infrastructure::Node',
    states: ['starting', 'idle', 'busy', 'draining', 'stopped'],
    initial: 'starting',
    transitions: [
      { source: 'starting', target: 'idle' },
      { source: 'idle', target: 'busy' },
      { source: 'busy', target: 'idle' },
      { source: 'idle', target: 'draining' },
      { source: 'draining', target: 'stopped' },
    ],
    exits: ['stopped'],
    parentType: recordType('Relations::NodeParent'),
    fields: {
      starting: [field('config', recordType('Config::Worker'))],
      busy: [
        field(
          'cpu',
          resourceReference('Resource::Cpu', 'ResourceData::CpuUsage'),
        ),
        field(
          'memory',
          resourceReference('Resource::Memory', 'ResourceData::MemoryUsage'),
        ),
        field(
          'network',
          resourceReference(
            'Resource::NetworkLink',
            'ResourceData::NetworkUsage',
          ),
        ),
        field('attempt', entityReference('Workload::Attempt')),
      ],
    },
  }),
  fsmEntity({
    name: 'Service::Database',
    parent: 'Infrastructure::Cluster',
    states: ['provisioning', 'serving', 'degraded', 'shutdown'],
    initial: 'provisioning',
    transitions: [
      { source: 'provisioning', target: 'serving' },
      { source: 'serving', target: 'serving' },
      { source: 'serving', target: 'degraded' },
      { source: 'degraded', target: 'serving' },
      { source: 'serving', target: 'shutdown' },
      { source: 'degraded', target: 'shutdown' },
    ],
    exits: ['shutdown'],
    fields: {
      provisioning: [field('config', recordType('Config::Database'))],
      serving: [
        field(
          'memory',
          resourceReference('Resource::Memory', 'ResourceData::MemoryUsage'),
        ),
        field(
          'disk',
          resourceReference('Resource::Disk', 'ResourceData::DiskUsage'),
        ),
        field('gateway', optional(entityReference('Service::Gateway'))),
      ],
    },
  }),
  fsmEntity({
    name: 'Service::Cache',
    parent: 'Infrastructure::Cluster',
    states: ['warming', 'serving', 'evicting', 'stopped'],
    initial: 'warming',
    transitions: [
      { source: 'warming', target: 'serving' },
      { source: 'serving', target: 'evicting' },
      { source: 'evicting', target: 'serving' },
      { source: 'serving', target: 'stopped' },
    ],
    exits: ['stopped'],
    fields: {
      serving: [
        field(
          'memory',
          resourceReference('Resource::Memory', 'ResourceData::MemoryUsage'),
        ),
      ],
    },
  }),
  fsmEntity({
    name: 'Service::Gateway',
    parent: 'Infrastructure::Region',
    states: ['starting', 'routing', 'throttled', 'stopped'],
    initial: 'starting',
    transitions: [
      { source: 'starting', target: 'routing' },
      { source: 'routing', target: 'routing' },
      { source: 'routing', target: 'throttled' },
      { source: 'throttled', target: 'routing' },
      { source: 'routing', target: 'stopped' },
    ],
    exits: ['stopped'],
    fields: {
      routing: [
        field(
          'network',
          resourceReference(
            'Resource::NetworkLink',
            'ResourceData::NetworkUsage',
          ),
        ),
        field('database', entityReference('Service::Database')),
        field('cache', optional(entityReference('Service::Cache'))),
      ],
    },
  }),
];

const observability = [
  simpleEntity('Observability::Trace', 'Workload::Attempt', {
    createdFields: [
      field('context', recordType('ObservabilityData::SpanContext')),
      field('job', entityReference('Workload::Job')),
      field('task', entityReference('Workload::Task')),
      field(
        'related',
        entityReference(null, {
          data: 'DynamicRecord',
          docs: 'Type-erased relation with dynamic payload.',
          metadata: { 'quent.example.edge-kind': 'diagnostic' },
        }),
      ),
    ],
    events: [
      event(
        'span_recorded',
        [field('span', recordType('Common::Envelope'))],
        'Multi',
      ),
    ],
  }),
  simpleEntity('Observability::MetricStream', 'Infrastructure::Node', {
    parentType: recordType('Relations::NodeParent'),
    createdFields: [
      field('metric_names', list('String')),
      field('dimensions', 'DynamicRecord'),
    ],
    events: [
      event(
        'batch_recorded',
        [
          field('timestamps', list('I64')),
          field('values', list('F64')),
        ],
        'Multi',
      ),
    ],
  }),
];

const security = [
  simpleEntity('Security::Identity', 'Workload::Tenant', {
    createdFields: [
      field('subject', 'String'),
      field('claims', recordType('SecurityData::Claims')),
    ],
    events: [
      event(
        'permissions_changed',
        [field('claims', recordType('SecurityData::Claims'))],
        'Multi',
      ),
    ],
  }),
  fsmEntity({
    name: 'Security::Session',
    parent: 'Security::Identity',
    states: ['opened', 'active', 'expired', 'revoked'],
    initial: 'opened',
    transitions: [
      { source: 'opened', target: 'active' },
      { source: 'active', target: 'active' },
      { source: 'active', target: 'expired' },
      { source: 'active', target: 'revoked' },
    ],
    exits: ['expired', 'revoked'],
    fields: {
      opened: [
        field('session_id', 'Uuid'),
        field('client', recordType('Common::Envelope')),
      ],
      active: [field('heartbeat_sequence', 'U64')],
    },
  }),
];

const entities = [
  platform,
  ...infrastructure,
  ...resources,
  ...workloads,
  ...services,
  ...observability,
  ...security,
];

export const stressSchema: Schema = {
  name: 'SchemaViewerStress',
  entities: entities.map((value) => [value.path, value]),
  records: records.map((value) => [value.path, value]),
  annotations: annotations(
    'Large schema covering schema and built-in constraint features.',
    [['quent.example.schema-version.v0.1.0', '"2026.07"']],
    {
      'quent.example.purpose': 'schema-viewer-stress-test',
      'quent.example.owner': 'quent',
    },
  ),
};

export const sampleSchema = stressSchema;
