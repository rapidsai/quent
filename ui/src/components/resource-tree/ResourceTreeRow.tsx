import { Engine } from '~quent/types/Engine';
import { Worker } from '~quent/types/Worker';
import { QueryGroup } from '~quent/types/QueryGroup';
import { ResourceGroup } from '~quent/types/ResourceGroup';
import { Resource } from '~quent/types/Resource';
import { Port } from '~quent/types/Port';
import { Operator } from '~quent/types/Operator';
import { Plan } from '~quent/types/Plan';
import { Query } from '~quent/types/Query';
import { EntityTypeValue } from '@/types';
import { Database, Folder, LineChart, LucideIcon, Network, Rocket } from 'lucide-react';

export type TreeTableItem = {
  id: string;
  type: string;
  entity: EntityTypeValue;
  icon: LucideIcon;
  children?: TreeTableItem[];
};

// Standardize default styles for resource tree rows
export const ResourceTreeRow = ({
  title,
  description,
  subText,
}: {
  title: string | React.ReactNode;
  description?: string;
  subText?: string;
}): React.ReactNode => {
  return (
    <div>
      <div>{typeof title === 'string' ? <span className="font-bold">{title}</span> : title}</div>
      {description && <div>{description}</div>}
      {subText && <div className="text-xs text-muted-foreground">{subText}</div>}
    </div>
  );
};

// Move to another place
const EngineRow = ({ entity }: { entity: Engine }): React.ReactNode => {
  const { implementation } = entity;
  const implementationName = implementation?.name ?? 'Unknown';
  const implementationVersion = implementation?.version ?? 'Unknown';
  return (
    <ResourceTreeRow
      title={
        <div>
          <span className="font-bold">Engine:</span> {entity.name}
        </div>
      }
      description={`${implementationName} (${implementationVersion})`}
      subText={entity.id}
    />
  );
};

const QueryGroupRow = ({ entity }: { entity: QueryGroup }): React.ReactNode => {
  return <ResourceTreeRow title={entity.name} subText={entity.id} />;
};

const ResourceGroupRow = ({ entity }: { entity: ResourceGroup }): React.ReactNode => {
  return <ResourceTreeRow title={entity.instance_name} subText={entity.id} />;
};
const WorkerRow = ({ entity }: { entity: Worker }): React.ReactNode => {
  return <ResourceTreeRow title={entity.name} subText={entity.id} />;
};
const ResourceRow = ({ entity }: { entity: Resource }): React.ReactNode => {
  return (
    <ResourceTreeRow
      title={
        <div>
          <span className="font-bold">{entity.instance_name}</span>{' '}
        </div>
      }
      subText={entity.id}
    />
  );
};
const PortRow = ({ entity }: { entity: Port }): React.ReactNode => {
  return <ResourceTreeRow title={entity.name ?? 'Port'} subText={entity.id} />;
};
const OperatorRow = ({ entity }: { entity: Operator }): React.ReactNode => {
  return <ResourceTreeRow title={entity.name ?? 'Operator'} subText={entity.id} />;
};
const PlanRow = ({ entity }: { entity: Plan }): React.ReactNode => {
  return <ResourceTreeRow title={entity.name ?? 'Plan'} subText={entity.id} />;
};
const QueryRow = ({ entity }: { entity: Query }): React.ReactNode => {
  return <ResourceTreeRow title={entity.name ?? 'Query'} subText={entity.id} />;
};

export const getRowForEntity = (item: TreeTableItem): React.ReactNode => {
  const { type, id, entity } = item;
  switch (type) {
    case 'Engine':
      return <EngineRow entity={entity as Engine} />;
    case 'QueryGroup':
      return <QueryGroupRow entity={entity as QueryGroup} />;
    case 'ResourceGroup':
      return <ResourceGroupRow entity={entity as ResourceGroup} />;
    case 'Worker':
      return <WorkerRow entity={entity as Worker} />;
    case 'Resource':
      return <ResourceRow entity={entity as Resource} />;
    case 'Port':
      return <PortRow entity={entity as Port} />;
    case 'Operator':
      return <OperatorRow entity={entity as Operator} />;
    case 'Plan':
      return <PlanRow entity={entity as Plan} />;
    case 'Query':
      return <QueryRow entity={entity as Query} />;
    default:
      return (
        <div>
          <span className="font-extrabold mr-1">{type}</span>
          {`(${id})`}
        </div>
      );
  }
};

export const getIconForType = (type: string): LucideIcon => {
  switch (type) {
    case 'Engine':
      return Database;
    case 'QueryGroup':
    case 'ResourceGroup':
      return Folder;
    case 'Network':
      return Network;
    case 'Worker':
      return Rocket;
    case 'Resource':
      return LineChart;
    default:
      return Database;
  }
};
