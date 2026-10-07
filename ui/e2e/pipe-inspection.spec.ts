// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from '@playwright/test';

const engine = '00000000-0000-0000-0000-000000000001';
const query = '00000000-0000-0000-0000-000000000004';

test('keyboard pipe inspection preserves operator filters and renders nested endpoint data', async ({
  page,
}) => {
  await page.route(`**/api/engines/${engine}/query/${query}`, async route => {
    const response = await route.fetch();
    const bundle = await response.json();
    for (const operator of Object.values(bundle.entities.operators) as Array<
      Record<string, unknown>
    >) {
      operator.custom_attributes = [
        {
          key: 'Decomposition',
          value: {
            Struct: [
              {
                key: 'Second stage',
                value: { Struct: [{ key: 'expression', value: { String: 'price * discount' } }] },
              },
              {
                key: 'First stage',
                value: { Struct: [{ key: 'expression', value: { String: 'quantity > 0' } }] },
              },
            ],
          },
        },
      ];
    }
    for (const port of Object.values(bundle.entities.ports) as Array<Record<string, unknown>>) {
      port.statistics = {
        custom_statistics: [
          {
            key: 'Volume',
            value: {
              Struct: [
                { key: 'bytes', value: { U64: 1024 } },
                { key: 'rows', value: { U64: 10 } },
              ],
            },
          },
        ],
      };
    }
    await route.fulfill({ response, json: bundle });
  });
  await page.goto(`/profile/engine/${engine}/query/${query}/timeline`);
  const node = page.locator('.react-flow__node').first();
  await expect(node).toBeVisible();
  await node.click();
  await expect(page.getByRole('heading', { name: 'Decomposition' }).first()).toBeVisible();
  const headings = page.getByRole('heading').filter({ hasText: /^(Second stage|First stage)$/ });
  expect((await headings.allTextContents()).slice(0, 2)).toEqual(['Second stage', 'First stage']);
  await page.screenshot({ path: '/tmp/quent-operator-decomposition.png', fullPage: true });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('combobox', { name: 'Edge width', exact: true }).click();
  await page.getByRole('option', { name: 'Volume › bytes', exact: true }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect
    .poll(async () =>
      Number(
        await page
          .locator('.react-flow__edge-path')
          .first()
          .evaluate(path => getComputedStyle(path).strokeWidth.replace('px', ''))
      )
    )
    .toBeGreaterThan(2);
  const selectedBefore = await page.locator('.react-flow__node.selected').count();
  const pipe = page.getByRole('button', { name: /^Inspect pipe / }).first();
  await pipe.focus();
  await pipe.press('Enter');
  await expect(page.getByText('Pipe Details', { exact: true })).toBeVisible();
  await expect(
    page.getByRole('region', { name: 'Sending port' }).getByRole('heading', { name: 'Volume' })
  ).toBeVisible();
  await expect(
    page.getByRole('region', { name: 'Receiving port' }).getByText('1.00 KiB')
  ).toBeVisible();
  await expect(pipe).toHaveAttribute('aria-pressed', 'true');
  expect(await page.locator('.react-flow__node.selected').count()).toBe(selectedBefore);
  await page.screenshot({ path: '/tmp/quent-pipe-inspection.png', fullPage: true });
  await node.click();
  await expect(page.getByText('Operator Details', { exact: true })).toBeVisible();
});

test('detail ports link to DAG edges and operators, and the panel resizes beyond half height', async ({
  page,
}) => {
  const ports: Record<string, { operator_id: string; instance_name: string }> = {};
  await page.route(`**/api/engines/${engine}/query/${query}`, async route => {
    const response = await route.fetch();
    const bundle = await response.json();
    Object.assign(ports, bundle.entities.ports);
    for (const [id, operator] of Object.entries(bundle.entities.operators) as Array<
      [string, Record<string, unknown>]
    >) {
      const ownedPorts = Object.entries(ports).filter(([, port]) => port.operator_id === id);
      // Include the same names in both directions: matching must also check the edge endpoint.
      operator.statistics = {
        custom_statistics: ['Inputs', 'Outputs'].map(key => ({
          value: {
            key,
            value: {
              Struct: ownedPorts.map(([, port]) => ({
                key: port.instance_name,
                value: {
                  Struct: [
                    { key: 'structural_role', value: { String: port.instance_name } },
                    { key: 'rows', value: { U64: 10 } },
                  ],
                },
              })),
            },
          },
          quantity: null,
        })),
      };
    }
    await route.fulfill({ response, json: bundle });
  });
  await page.goto(`/profile/engine/${engine}/query/${query}/timeline`);
  const edge = page.locator('.react-flow__edge').first();
  // Straight vertical SVG paths can have zero bounding-box width despite a visible stroke.
  await expect(edge).toBeAttached();
  const edgeId = (await edge.getAttribute('data-id'))!;
  const sourcePortId = Object.keys(ports).find(id => edgeId.startsWith(`${id}-`))!;
  const targetPortId = edgeId.slice(sourcePortId.length + 1);
  const source = page.locator(`.react-flow__node[data-id="${ports[sourcePortId].operator_id}"]`);
  const target = page.locator(`.react-flow__node[data-id="${ports[targetPortId].operator_id}"]`);
  const sourceLabel = (await source.innerText()).trim();
  await source.click();
  const output = page
    .getByRole('button', {
      name: `Inspect outputs ${ports[sourcePortId].instance_name} pipe`,
      exact: true,
    })
    .first();
  await expect(output).toBeVisible();
  await output.hover();
  await expect(edge.locator('.react-flow__edge-path')).toHaveCSS('opacity', '1');
  await expect
    .poll(() =>
      edge.locator('.react-flow__edge-path').evaluate(path => (path as SVGElement).style.filter)
    )
    .toContain('drop-shadow');
  const selectedBefore = await page.locator('.react-flow__node.selected').count();
  await output.click();
  await expect(page.getByText('Pipe Details', { exact: true })).toBeVisible();
  expect(await page.locator('.react-flow__node.selected').count()).toBe(selectedBefore);
  const sending = page.getByRole('region', { name: 'Sending port' });
  await expect(sending.getByText(sourceLabel, { exact: true })).toBeVisible();
  await sending.hover();
  await expect(source.locator('.shadow-glow')).toBeVisible();
  await page.getByRole('region', { name: 'Receiving port' }).hover();
  await expect(target.locator('.shadow-glow')).toBeVisible();
  await expect(source.locator('.shadow-glow')).toHaveCount(0);

  await page.getByRole('region', { name: 'Receiving port' }).click();
  await expect(page.getByText('Operator Details', { exact: true })).toBeVisible();
  await expect(target.locator('.shadow-glow')).toBeVisible();
  await expect(source.locator('.shadow-glow')).toHaveCount(0);
  const input = page
    .getByRole('button', {
      name: `Inspect inputs ${ports[targetPortId].instance_name} pipe`,
      exact: true,
    })
    .first();
  await expect(input).toBeVisible();
  await input.focus();
  await expect
    .poll(() =>
      edge.locator('.react-flow__edge-path').evaluate(path => (path as SVGElement).style.filter)
    )
    .toContain('drop-shadow');
  await input.press('Space');
  await expect(page.getByText('Pipe Details', { exact: true })).toBeVisible();

  const details = page.locator('#operator-details');
  const group = details.locator('..');
  const box = (await group.boundingBox())!;
  const handle = group.getByRole('separator');
  const handleBox = (await handle.boundingBox())!;
  await page.mouse.move(handleBox.x + handleBox.width / 2, handleBox.y + handleBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(handleBox.x + handleBox.width / 2, box.y + box.height * 0.2, { steps: 10 });
  await page.mouse.up();
  await expect
    .poll(async () => (await details.boundingBox())!.height)
    .toBeGreaterThan(box.height * 0.7);
  await page.screenshot({ path: '/tmp/quent-linked-details.png', fullPage: true });
  const sendingOperator = page
    .getByRole('region', { name: 'Sending port' })
    .getByRole('button', { name: /^Show .* operator details$/ });
  await sendingOperator.focus();
  await expect(source.locator('.shadow-glow')).toBeVisible();
  await sendingOperator.press('Enter');
  await expect(page.getByText('Operator Details', { exact: true })).toBeVisible();
  await expect(source.locator('.shadow-glow')).toBeVisible();
  await expect(target.locator('.shadow-glow')).toHaveCount(0);
});
