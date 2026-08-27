// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Card } from '@quent/components';
import { cn, formatAttributeValue, unwrapTaggedValue } from '@quent/utils';
import type { DynamicAttribute } from '@quent/utils';

interface TransitionAttributesProps {
  attributes: DynamicAttribute[];
  derivedAttributes: DynamicAttribute[];
  operatorLabel: (id: string) => string;
}

export function TransitionAttributes({
  attributes,
  derivedAttributes,
  operatorLabel,
}: TransitionAttributesProps) {
  if (attributes.length === 0 && derivedAttributes.length === 0) {
    return null;
  }

  return (
    <div className="mt-1.5 space-y-1.5">
      <AttributeGroup title="Attributes" attributes={attributes} operatorLabel={operatorLabel} />
      <AttributeGroup
        title="Derived attributes"
        attributes={derivedAttributes}
        operatorLabel={operatorLabel}
        derived
      />
    </div>
  );
}

function AttributeGroup({
  title,
  attributes,
  operatorLabel,
  derived,
}: {
  title: string;
  attributes: DynamicAttribute[];
  operatorLabel: (id: string) => string;
  derived?: boolean;
}) {
  if (attributes.length === 0) {
    return null;
  }

  return (
    <Card className="bg-muted/20 p-2 shadow-none">
      <h4 className="text-xs font-medium">{title}</h4>
      <dl className={cn('mt-1 space-y-0.5 border-t pt-1 text-xs', derived && 'italic')}>
        {attributes.map((attribute, index) => {
          const { label, value } = resolveAttributeDisplay(attribute, operatorLabel);
          return (
            <div key={`${attribute.key}-${index}`} className="flex justify-between gap-3">
              <dt className="text-muted-foreground">{label}</dt>
              <dd className="tabular-nums text-right">{value}</dd>
            </div>
          );
        })}
      </dl>
    </Card>
  );
}

function resolveAttributeDisplay(
  attribute: DynamicAttribute,
  operatorLabel: (id: string) => string
): { label: string; value: string } {
  if (attribute.key === 'operator_id') {
    const raw = unwrapTaggedValue(attribute.value);
    if (typeof raw === 'string') {
      return { label: 'operator', value: operatorLabel(raw) };
    }
  }
  return { label: attribute.key, value: formatAttributeValue(attribute.key, attribute.value) };
}
