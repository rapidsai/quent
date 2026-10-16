// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test, type Page } from '@playwright/test';

const engine = '00000000-0000-0000-0000-000000000001';
const query = '00000000-0000-0000-0000-000000000004';

const pipeDetailsTab = (page: Page) => page.getByRole('tab', { name: 'Pipes' });
const operatorTab = (page: Page) => page.getByRole('tab', { name: 'Operators' });

test('operators and pipes are selected independently of each other', async ({ page }) => {
  await page.goto(`/profile/engine/${engine}/query/${query}/timeline`);
  const node = page.locator('.react-flow__node').first();
  await expect(node).toBeVisible();
  const clearAll = page.getByRole('button', { name: 'Clear all filters' });
  // Only selected nodes get `scale-110`; the hover glow does not.
  const selectedLook = node.locator('.scale-110');

  await node.click();
  await expect(clearAll).toBeVisible();
  await expect(selectedLook).toHaveCount(1);

  const pipe = page.getByRole('button', { name: /^Inspect pipe / }).first();
  await pipe.focus();
  await pipe.press('Enter');
  // The pipe gets its own tab next to the operator's.
  await expect(pipeDetailsTab(page)).toBeVisible();
  await expect(operatorTab(page)).toBeVisible();
  // Inspecting a pipe leaves the operator filter alone, so the node must still look selected.
  await expect(clearAll).toBeVisible();
  await expect(selectedLook).toHaveCount(1);

  // Clicking a selected node deselects only that node. The pipe stays inspected.
  await node.click();
  await expect(clearAll).toHaveCount(0);
  await expect(selectedLook).toHaveCount(0);
  await expect(pipe).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('region', { name: 'Sending port' })).toBeVisible();
});

test('selecting a pipe de-emphasizes everything else without selecting any operator', async ({
  page,
}) => {
  await page.goto(`/profile/engine/${engine}/query/${query}/timeline`);
  const nodes = page.locator('.react-flow__node');
  await expect(nodes.first()).toBeVisible();
  const dimmed = page.locator('.react-flow__node [class*="opacity-35"]');
  await expect(dimmed).toHaveCount(0);

  const pipe = page.getByRole('button', { name: /^Inspect pipe / }).first();
  await pipe.focus();
  await pipe.press('Enter');
  await expect(page.getByRole('region', { name: 'Sending port' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Clear all filters' })).toHaveCount(0);
  await expect(page.locator('.react-flow__node .scale-110')).toHaveCount(0);
  // Every node is dimmed, the pipe's endpoints included.
  await expect(dimmed).toHaveCount(await nodes.count());

  await pipe.press('Enter');
  await expect(dimmed).toHaveCount(0);
});

test('a pipe closes when activated again, leaving operators alone', async ({ page }) => {
  await page.goto(`/profile/engine/${engine}/query/${query}/timeline`);
  const node = page.locator('.react-flow__node').first();
  await expect(node).toBeVisible();
  // Keep an operator selected so there is something to fall back to.
  await node.click();
  const clearAll = page.getByRole('button', { name: 'Clear all filters' });
  await expect(clearAll).toBeVisible();

  const pipe = page.getByRole('button', { name: /^Inspect pipe / }).first();
  await pipe.focus();
  await pipe.press('Enter');
  await expect(pipeDetailsTab(page)).toBeVisible();
  await expect(pipe).toHaveAttribute('aria-pressed', 'true');

  await pipe.press('Enter');
  await expect(pipeDetailsTab(page)).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Sending port' })).toHaveCount(0);
  await expect(pipe).toHaveAttribute('aria-pressed', 'false');
  await expect(clearAll).toBeVisible();
});

test('pipes are deselected one at a time or all at once from the details panel', async ({
  page,
}) => {
  await page.goto(`/profile/engine/${engine}/query/${query}/timeline`);
  const node = page.locator('.react-flow__node').first();
  await expect(node).toBeVisible();
  // Keep an operator selected: clearing pipes must never touch it.
  await node.click();
  const clearAll = page.getByRole('button', { name: 'Clear all filters' });
  await expect(clearAll).toBeVisible();

  const pipes = page.getByRole('button', { name: /^Inspect pipe / });
  const first = pipes.nth(0);
  const second = pipes.nth(1);
  for (const pipe of [first, second]) {
    await pipe.focus();
    await pipe.press('Enter');
    await expect(pipe).toHaveAttribute('aria-pressed', 'true');
  }
  await expect(page.getByRole('region', { name: 'Sending port' })).toHaveCount(2);

  // Pressing a selected pipe again deselects only that pipe.
  await first.press('Enter');
  await expect(page.getByRole('region', { name: 'Sending port' })).toHaveCount(1);
  await expect(first).toHaveAttribute('aria-pressed', 'false');
  await expect(second).toHaveAttribute('aria-pressed', 'true');

  // Clear selection removes the remaining pipe and the selected operator.
  await page.getByRole('button', { name: 'Clear selection' }).click();
  await expect(second).toHaveAttribute('aria-pressed', 'false');
  await expect(pipeDetailsTab(page)).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Sending port' })).toHaveCount(0);
  await expect(node.locator('.scale-110')).toHaveCount(0);
});

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
  const clearAll = page.getByRole('button', { name: 'Clear all filters' });
  await expect(clearAll).toBeVisible();
  const pipe = page.getByRole('button', { name: /^Inspect pipe / }).first();
  await pipe.focus();
  await pipe.press('Enter');
  await expect(pipeDetailsTab(page)).toBeVisible();
  await expect(
    page.getByRole('region', { name: 'Sending port' }).getByRole('heading', { name: 'Volume' })
  ).toBeVisible();
  await expect(
    page.getByRole('region', { name: 'Receiving port' }).getByText('1.00 KiB')
  ).toBeVisible();
  await expect(pipe).toHaveAttribute('aria-pressed', 'true');
  await expect(clearAll).toBeVisible();
  await node.click();
  await expect(clearAll).toHaveCount(0);
  await expect(pipe).toHaveAttribute('aria-pressed', 'true');
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
  const clearAll = page.getByRole('button', { name: 'Clear all filters' });
  await expect(clearAll).toBeVisible();
  await output.click();
  await expect(pipeDetailsTab(page)).toBeVisible();
  await expect(clearAll).toBeVisible();
  const sending = page.getByRole('region', { name: 'Sending port' });
  await expect(sending.getByText(sourceLabel, { exact: true })).toBeVisible();
  // The source is still selected and always glows, so only the target shows the hover highlight.
  await sending.hover();
  await expect(target.locator('.shadow-glow')).toHaveCount(0);
  await page.getByRole('region', { name: 'Receiving port' }).hover();
  await expect(target.locator('.shadow-glow')).toBeVisible();
  // Selecting an endpoint adds its operator to the selection and keeps the pipe inspected.
  await page.getByRole('region', { name: 'Receiving port' }).click();
  await expect(operatorTab(page)).toBeVisible();
  await expect(pipeDetailsTab(page)).toBeVisible();
  await expect(target.locator('.shadow-glow')).toBeVisible();
  await expect(source.locator('.shadow-glow')).toBeVisible();
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
  await expect(pipeDetailsTab(page)).toBeVisible();

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
  // Selecting the endpoint operator earlier moved the panel to the operator tab.
  await pipeDetailsTab(page).click();
  const sendingOperator = page
    .getByRole('region', { name: 'Sending port' })
    .getByRole('button', { name: /^Show .* operator details$/ });
  await sendingOperator.focus();
  await expect(source.locator('.shadow-glow')).toBeVisible();
  await sendingOperator.press('Enter');
  await expect(source.locator('.shadow-glow')).toBeVisible();
  await expect(target.locator('.shadow-glow')).toBeVisible();
});
