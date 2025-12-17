import type { QueryPlanTransformer, DAGData } from './types';
import { prestoTransformer } from './presto-transformer';
import { prestoPhysicalTransformer } from './presto-physical-transformer';
import { queryEngineTransformer } from './query-engine-transformer';

class TransformerRegistry {
  private transformers = new Map<string, QueryPlanTransformer>();

  constructor() {
    this.register(prestoTransformer);
    this.register(prestoPhysicalTransformer);
    this.register(queryEngineTransformer);
  }

  register(transformer: QueryPlanTransformer): void {
    this.transformers.set(transformer.engineName, transformer);
  }

  get(engineName: string): QueryPlanTransformer | undefined {
    return this.transformers.get(engineName);
  }

  transform(engineName: string, plan: unknown): DAGData {
    const transformer = this.get(engineName);

    if (!transformer) {
      throw new Error(`No transformer registered for engine: ${engineName}`);
    }

    if (!transformer.validate(plan)) {
      throw new Error(`Invalid query plan format for engine: ${engineName}`);
    }

    return transformer.transform(plan);
  }

  autoTransform(plan: unknown): DAGData {
    for (const transformer of this.transformers.values()) {
      if (transformer.validate(plan)) {
        return transformer.transform(plan);
      }
    }

    throw new Error('Could not detect query plan engine format');
  }
}

export const transformerRegistry = new TransformerRegistry();
