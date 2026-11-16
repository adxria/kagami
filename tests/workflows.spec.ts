import { expect, test, type Page } from '@playwright/test';
async function rpc(page: Page, method: string, extra: Record<string, unknown> = {}) {
  return page.evaluate(
    async ({ method, extra }) => {
      const response = await fetch('/__test__/rpc', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'x-kagami-session': sessionStorage.kagamiTestSession,
        },
        body: JSON.stringify({ method, now: Date.now(), ...extra }),
      });
      const result = await response.json();
      if (result.error) throw new Error(result.error.message);
      return result.ok;
    },
    { method, extra },
  );
}
async function snapshot(page: Page) {
  const now = Date.now();
  const start = now - 7 * 86400_000;
  return rpc(page, 'read', {
    query: {
      weekStart: start,
      weekEnd: now + 7 * 86400_000,
      todayStart: start,
      todayEnd: now + 86400_000,
    },
  });
}
async function createContext(page: Page, name: string) {
  await page.getByRole('button', { name: 'New context', exact: true }).click();
  await page.getByRole('textbox', { name: 'Name', exact: true }).fill(name);
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
}
test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeEnabled();
});
test('search cycling, cancellation and assignment failures preserve the running recording', async ({
  page,
}) => {
  await createContext(page, 'Work');
  await page.getByRole('button', { name: 'Start', exact: true }).click();
  const first = (await snapshot(page)).running;
  const picker = page.getByRole('combobox', { name: 'Search contexts' });
  await picker.click();
  expect(
    await picker.evaluate((input: HTMLInputElement) => [input.selectionStart, input.selectionEnd]),
  ).toEqual([0, 4]);
  await picker.fill('unass');
  expect((await snapshot(page)).running).toEqual(first);
  await picker.press('Escape');
  await expect(picker).toHaveValue('Work');
  await picker.press('Tab');
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toBeFocused();
  await picker.click();
  const work = page.getByRole('option', { name: 'Work', exact: true });
  const unassigned = page.getByRole('option', { name: 'Unassigned', exact: true });
  await expect(picker).toHaveAttribute('aria-activedescendant', (await work.getAttribute('id'))!);
  await picker.press('Tab');
  await expect(picker).toBeFocused();
  await expect(picker).toHaveAttribute(
    'aria-activedescendant',
    (await unassigned.getAttribute('id'))!,
  );
  await picker.press('Shift+Tab');
  await expect(picker).toHaveAttribute('aria-activedescendant', (await work.getAttribute('id'))!);
  await picker.press('ArrowUp');
  await rpc(page, 'fail_change');
  await picker.press('Enter');
  await expect(page.getByRole('alert')).toContainText('Injected write failure');
  expect((await snapshot(page)).running).toEqual(first);
  await picker.press('Enter');
  await expect(picker).toHaveAttribute('aria-expanded', 'false');
  expect((await snapshot(page)).running).toMatchObject({
    id: first.id,
    startedAt: first.startedAt,
    contextId: null,
  });
  await rpc(page, 'reopen');
  await page.reload();
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Stop', exact: true }).click();
  await expect(picker).toHaveValue('Unassigned');
  const data = await snapshot(page);
  expect(data.running).toBeNull();
  expect(data.recordings).toHaveLength(1);
  expect(data.recordings[0].startedAt).toBe(first.startedAt);
});
test('locked choices require an exact search and remain available for completed corrections', async ({
  page,
}) => {
  const create = async (name: string, parentId: string | null = null) =>
    (
      await rpc(page, 'change', {
        change: { type: 'create_context', name, parentId, color: '#7FBCF5' },
      })
    ).id;
  const project = await create('Project');
  const locked = await create('Archive', project);
  const child = await create('Active child', project);
  await rpc(page, 'change', { change: { type: 'lock_context', id: locked, locked: true } });
  await rpc(page, 'change', { change: { type: 'lock_context', id: project, locked: true } });
  await page.reload();
  const picker = page.getByRole('combobox', { name: 'Search contexts' });
  await picker.click();
  await expect(page.getByRole('option', { name: /Tracking locked/ })).toHaveCount(0);
  await picker.fill('arch');
  await expect(page.getByRole('option', { name: /Tracking locked/ })).toHaveCount(0);
  await picker.fill('  aRCHIVE  ');
  await expect(page.getByRole('option', { name: /Tracking locked/ })).toBeDisabled();
  await picker.press('Enter');
  await expect(picker).toHaveAttribute('aria-expanded', 'true');
  await picker.press('Tab');
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeFocused();
  await picker.click();
  await picker.fill(' PROJECT / ARCHIVE ');
  await expect(page.getByRole('option', { name: /Tracking locked/ })).toBeDisabled();
  await picker.press('Escape');
  await picker.click();
  await picker.fill(' PROJECT ');
  await picker.press('Tab');
  const unlocked = page.getByRole('option', {
    name: 'Project / Active child',
    exact: true,
  });
  await expect(picker).toHaveAttribute(
    'aria-activedescendant',
    (await unlocked.getAttribute('id'))!,
  );
  await picker.press('Enter');
  await page.getByRole('button', { name: 'Start', exact: true }).click();
  expect((await snapshot(page)).running.contextId).toBe(child);
  await page.getByRole('button', { name: 'Stop', exact: true }).click();
  await page.getByRole('button', { name: '+ Add entry', exact: true }).click();
  const choices = page.getByRole('dialog').getByRole('combobox', { name: 'Context', exact: true });
  await expect(choices.locator(`option[value="${locked}"]`)).toHaveCount(0);
  await expect(choices.locator(`option[value="${child}"]`)).toHaveCount(1);
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await page.locator('.short-marker').click();
  await choices.selectOption(locked);
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect((await snapshot(page)).recordings[0].contextId).toBe(locked);
  await page.locator('.short-marker').click();
  await choices.selectOption(child);
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect((await snapshot(page)).recordings[0].contextId).toBe(child);
});
test('settings persist targeted changes and single-step rename undo', async ({ page }) => {
  await createContext(page, 'Parent');
  await createContext(page, 'Child');
  const name = page.getByRole('textbox', { name: 'Context name' });
  await name.fill('Renamed');
  await name.press('Enter');
  await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Settings', exact: true }).focus();
  await page.keyboard.press('Control+z');
  await expect(name).toHaveValue('Child');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const parent = (await snapshot(page)).contexts.find(
    (c: { name: string }) => c.name === 'Parent',
  ).id;
  await page.getByRole('combobox', { name: 'Parent context', exact: true }).selectOption(parent);
  const hex = page.getByRole('textbox', { name: 'HEX color' });
  await hex.fill('#000');
  await page.getByRole('button', { name: 'Apply color', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Lock tracking' })).toBeEnabled();
  await page.getByRole('checkbox', { name: 'Lock tracking' }).check();
  await expect(page.getByRole('checkbox', { name: 'Lock tracking' })).toBeChecked();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeFocused();
  await rpc(page, 'reopen');
  await page.reload();
  const child = (await snapshot(page)).contexts.find((c: { name: string }) => c.name === 'Child');
  expect(child).toMatchObject({ parentId: parent, color: '#000000', trackingLocked: true });
});
test('failed entry writes retain drafts; corrections preserve unchanged timestamp precision', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Previous week' }).click();
  await page.getByRole('button', { name: '+ Add entry', exact: true }).click();
  await page.getByRole('textbox', { name: 'Duration', exact: true }).fill('00:10:03');
  await rpc(page, 'fail_change');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Injected write failure');
  await expect(page.getByRole('textbox', { name: 'Duration', exact: true })).toHaveValue(
    '00:10:03',
  );
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  const original = (await snapshot(page)).recordings[0];
  expect(original.endedAt - original.startedAt).toBe(603000);
  await page.locator('.short-marker').click();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  const corrected = (await snapshot(page)).recordings[0];
  expect(corrected).toEqual(original);
});
test('a committed creation closes once when the following refresh fails', async ({ page }) => {
  await page.getByRole('button', { name: '+ Add entry', exact: true }).click();
  await rpc(page, 'fail_read');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('alert')).toContainText('already stored');
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeDisabled();
  expect((await snapshot(page)).recordings).toHaveLength(1);
  await page.getByRole('button', { name: 'Refresh', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeEnabled();
  expect((await snapshot(page)).recordings).toHaveLength(1);
});
test('confirmed subtree deletion removes a running child and historical entries', async ({
  page,
}) => {
  await createContext(page, 'Parent');
  await page.locator('.context-row').filter({ hasText: 'Parent' }).hover();
  await page.getByRole('button', { name: 'Add child to Parent', exact: true }).click();
  await page.getByRole('textbox', { name: 'Name', exact: true }).fill('Child');
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Start', exact: true }).click();
  await page.locator('.context-link').filter({ hasText: 'Parent' }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Delete context', exact: true }).click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect((await snapshot(page)).running).not.toBeNull();
  await page.getByRole('button', { name: 'Delete context', exact: true }).click();
  await page.getByRole('button', { name: 'Delete permanently', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  const data = await snapshot(page);
  expect(data.contexts).toHaveLength(0);
  expect(data.running).toBeNull();
  expect(data.recordings).toHaveLength(0);
});
test('rapid week navigation ignores obsolete reads and keeps zoom', async ({ page }) => {
  const monday = await page.evaluate(() => {
    const d = new Date();
    d.setHours(0, 0, 0, 0);
    d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
    return d.getTime();
  });
  for (const [name, offset] of [
    ['Older', 14],
    ['Previous', 7],
  ] as const) {
    const context = await rpc(page, 'change', {
      change: { type: 'create_context', name, parentId: null, color: '#6688AA' },
    });
    const startedAt = monday - offset * 86400_000 + 12 * 3600_000;
    await rpc(page, 'change', {
      change: {
        type: 'create_recording',
        contextId: context.id,
        startedAt,
        endedAt: startedAt + 3600_000,
      },
    });
  }
  await page.getByRole('button', { name: 'Zoom in', exact: true }).click();
  let release: () => void = () => {};
  const held = new Promise<void>((done) => (release = done));
  let intercepted = false;
  await page.route('**/__test__/rpc', async (route) => {
    const data = route.request().postDataJSON();
    if (data.method === 'read' && !intercepted) {
      intercepted = true;
      await held;
    }
    await route.continue();
  });
  await page.getByRole('button', { name: 'Previous week' }).click();
  await page.getByRole('button', { name: 'Next week' }).click();
  await page.getByRole('button', { name: 'Previous week' }).click();
  await page.getByRole('button', { name: 'Previous week' }).click();
  release();
  await expect(page.getByRole('button', { name: 'Reset zoom' })).toHaveText('150%');
  await expect(page.locator('.calendar-entry')).toHaveCount(1);
  await expect(page.locator('.calendar-entry')).toContainText('Older');
});

test('tree drops persist both hierarchy and sibling placement', async ({ page }) => {
  await createContext(page, 'A');
  await createContext(page, 'B');
  await createContext(page, 'C');
  const a = page
    .locator('.context-row')
    .filter({ has: page.getByRole('button', { name: 'A', exact: true }) });
  const b = page
    .locator('.context-row')
    .filter({ has: page.getByRole('button', { name: 'B', exact: true }) });
  const c = page
    .locator('.context-row')
    .filter({ has: page.getByRole('button', { name: 'C', exact: true }) });
  await c.dragTo(a);
  await expect
    .poll(async () => {
      const data = await snapshot(page);
      return data.contexts.find((x: { name: string }) => x.name === 'C').parentId;
    })
    .toBe((await snapshot(page)).contexts.find((x: { name: string }) => x.name === 'A').id);
  await b.dragTo(a, { targetPosition: { x: 30, y: 39 } });
  await expect
    .poll(async () => {
      const data = await snapshot(page);
      return data.contexts
        .filter((x: { parentId: string | null }) => x.parentId === null)
        .sort((x: { position: number }, y: { position: number }) => x.position - y.position)
        .map((x: { name: string }) => x.name);
    })
    .toEqual(['A', 'B']);
  const saved = (await snapshot(page)).contexts;
  await rpc(page, 'fail_change');
  await b.dragTo(a);
  await expect(page.getByRole('alert')).toContainText('Injected write failure');
  expect((await snapshot(page)).contexts).toEqual(saved);
  // Blank space below the hint accepts a top-level drop too.
  const source = (await c.boundingBox())!;
  await page.mouse.move(source.x + 60, source.y + source.height / 2);
  await page.mouse.down();
  await page.mouse.move(source.x + 76, source.y + source.height / 2, { steps: 4 });
  const root = (await page.locator('.root-drop').boundingBox())!;
  await page.mouse.move(root.x + 60, root.y + root.height + 24, { steps: 8 });
  await page.mouse.up();
  await expect
    .poll(async () =>
      (await snapshot(page)).contexts
        .filter((x: { parentId: string | null }) => x.parentId === null)
        .sort((x: { position: number }, y: { position: number }) => x.position - y.position)
        .map((x: { name: string }) => x.name),
    )
    .toEqual(['A', 'B', 'C']);
  // A cancelled drop over an editor must not insert the dragged context ID.
  await a.dragTo(page.getByRole('textbox', { name: 'Context name' }));
  await expect(page.getByRole('textbox', { name: 'Context name' })).toHaveValue('C');
  await expect(page.locator('.root-drop')).toHaveCount(0);
  await rpc(page, 'reopen');
  await page.reload();
  expect(
    (await snapshot(page)).contexts.find((x: { name: string }) => x.name === 'C'),
  ).toMatchObject({ parentId: null, position: 2 });
});
