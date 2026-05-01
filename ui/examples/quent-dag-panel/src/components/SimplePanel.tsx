import * as React from 'react';
import { PanelProps } from '@grafana/data';
import { DagPanelOptions } from 'types';
import { css, cx } from '@emotion/css';
import { useStyles2 } from '@grafana/ui';
import { DEFAULT_STALE_TIME, setApiBaseUrl, useQueryBundle } from '@quent/client';
import { DAGChart, getPlanDAG, getTreeData, type DAGData } from '@quent/components';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useMemo } from 'react';

interface Props extends PanelProps<DagPanelOptions> {}

const DEFAULT_API_BASE_URL = 'http://localhost:8080/api';
const DEFAULT_ENGINE_ID = '019cff13-5304-7293-9b8b-16830808274b';
const DEFAULT_QUERY_ID = '019cff13-5adf-75c1-a40c-57fbee4392c4';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: DEFAULT_STALE_TIME,
      refetchOnWindowFocus: false,
    },
  },
});

const emptyDagData: DAGData = { nodes: [], edges: [], queryData: [] };

const getStyles = () => {
  return {
    wrapper: css`
      width: 100%;
      height: 100%;
      overflow: hidden;
    `,
  };
};

function QuentDagPanel({ options, width, height }: { options: DagPanelOptions; width: number; height: number }) {
  const apiBaseUrl = options.apiBaseUrl?.trim() || DEFAULT_API_BASE_URL;
  const engineId = options.engineId?.trim() || DEFAULT_ENGINE_ID;
  const queryId = options.queryId?.trim() || DEFAULT_QUERY_ID;
  setApiBaseUrl(apiBaseUrl);

  const {
    data: queryBundle,
    isLoading,
    error,
  } = useQueryBundle({
    engineId,
    queryId,
  });

  const dagData = useMemo<DAGData>(() => {
    if (!queryBundle) {
      return emptyDagData;
    }
    try {
      const planId = queryBundle.plan_tree.id;
      return {
        ...getPlanDAG(queryBundle, planId),
        queryData: getTreeData(queryBundle),
      };
    } catch {
      return emptyDagData;
    }
  }, [queryBundle]);

  if (isLoading) {
    return (
      <div style={{ width, height, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        Loading query bundle...
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          width,
          height,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          textAlign: 'center',
          padding: 16,
        }}
      >
        Failed to load query bundle from <code>{apiBaseUrl}</code> for engine <code>{engineId}</code> and query{' '}
        <code>{queryId}</code>.
      </div>
    );
  }

  return (
    <div style={{ width, height }}>
      <DAGChart data={dagData} isDark />
    </div>
  );
}

export const SimplePanel: React.FC<Props> = ({ options, width, height }) => {
  const styles = useStyles2(getStyles);

  return (
    <div
      className={cx(
        styles.wrapper,
        css`
          width: ${width}px;
          height: ${height}px;
        `
      )}
    >
      <QueryClientProvider client={queryClient}>
        <QuentDagPanel options={options} width={width} height={height} />
      </QueryClientProvider>
    </div>
  );
};
