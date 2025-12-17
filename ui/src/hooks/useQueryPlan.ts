import { useQuery } from '@tanstack/react-query';
import { transformerRegistry } from '@/services/query-plan/transformer-registry';
import { fetchLocalQueryPlan, fetchQueryPlan } from '@/services/api';
import type { DAGData, QueryPlanSource } from '@/services/query-plan/types';

interface UseQueryPlanOptions {
  source: QueryPlanSource;
  engineName?: string;
}

export const useQueryPlan = ({ source, engineName }: UseQueryPlanOptions) => {
  return useQuery({
    queryKey: ['queryPlan', source],
    queryFn: async (): Promise<DAGData> => {
      // temporary until we can fetch query plans from api
      let rawPlan;

      if (source.type === 'local') {
        rawPlan = await fetchLocalQueryPlan(source.path);
      } else {
        rawPlan = await fetchQueryPlan(source.engineId, source.queryId);
      }

      // Transform to DAG format
      if (engineName) {
        return transformerRegistry.transform(engineName, rawPlan);
      } else {
        return transformerRegistry.autoTransform(rawPlan);
      }
    },
    staleTime: 5 * 60 * 1000,
    retry: 2,
  });
};
