import { PanelPlugin } from '@grafana/data';
import { DagPanelOptions } from './types';
import { SimplePanel } from './components/SimplePanel';

export const plugin = new PanelPlugin<DagPanelOptions>(SimplePanel).setPanelOptions((builder) => {
  return builder
    .addTextInput({
      path: 'apiBaseUrl',
      name: 'API Base URL',
      description: 'Base URL for Quent API requests',
      defaultValue: 'http://localhost:8080/api',
    })
    .addTextInput({
      path: 'engineId',
      name: 'Engine ID',
      description: 'Quent engine ID to fetch query bundle data',
      defaultValue: '019cff13-5304-7293-9b8b-16830808274b',
    })
    .addTextInput({
      path: 'queryId',
      name: 'Query ID',
      description: 'Quent query ID to visualize as DAG',
      defaultValue: '019cff13-5adf-75c1-a40c-57fbee4392c4',
    });
});
