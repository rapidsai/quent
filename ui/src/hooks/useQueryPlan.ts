import { useQuery } from '@tanstack/react-query';
import { transformerRegistry } from '@/services/query-plan/transformer-registry';
import { DEFEAULT_STALE_TIME, fetchQueryBundle } from '@/services/api';
import type { DAGData, QueryPlanSource } from '@/services/query-plan/types';

interface UseQueryPlanOptions {
  source: QueryPlanSource;
  engineName?: string;
}

export const useQueryPlan = ({ source }: UseQueryPlanOptions) => {
  return useQuery({
    queryKey: ['queryPlan', source],
    queryFn: async (): Promise<DAGData> => {
      // temporary until we can fetch query plans from api
      const queryBundle = await fetchQueryBundle(source.engineId, source.queryId);
      return transformerRegistry.transform('quent', queryBundle);
    },
    staleTime: DEFEAULT_STALE_TIME,
    retry: 2,
  });
};
