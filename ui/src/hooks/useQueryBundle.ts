import { DEFEAULT_STALE_TIME, fetchQueryBundle } from '@/services/api';
import { useQuery } from '@tanstack/react-query';
import type { QueryBundle } from '~quent/types/QueryBundle';

interface UseQueryBundleOptions {
  engineId: string;
  queryId: string;
}

export const useQueryBundle = ({ engineId, queryId }: UseQueryBundleOptions) => {
  return useQuery({
    queryKey: ['queryBundle', engineId, queryId],
    queryFn: async (): Promise<QueryBundle> => {
      return fetchQueryBundle(engineId, queryId) as Promise<QueryBundle>;
    },
    staleTime: DEFEAULT_STALE_TIME,
    retry: 2,
  });
};
