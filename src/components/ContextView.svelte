<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { Controller } from '../lib/controller.svelte';
  import { errorMessage, type Context, type Change } from '../lib/types';
  import {
    colorStyle,
    contextColor,
    contextPath,
    contextRows,
    normalizeColor,
    subtreeIds,
  } from '../lib/contexts';
  import { duration } from '../lib/time';
  import ColorField from './ColorField.svelte';
  let {
    app,
    context,
    ondelete,
  }: { app: Controller; context: Context | null; ondelete: () => void } = $props();
  let name = $state(untrack(() => context?.name ?? '')),
    previous = $state<string | null>(null),
    renameError = $state(''),
    settingsError = $state(''),
    open = $state(false),
    color = $state(untrack(() => context?.color ?? ''));
  let settingsButton = $state<HTMLButtonElement>(null!);
  let panel = $state<HTMLElement>(null!);
  const stats = $derived(
    app.totals(context ? { kind: 'context', id: context.id } : { kind: 'unassigned' }),
  );
  const descendants = $derived(context ? subtreeIds(app.contexts, context.id) : new Set<string>());
  const parents = $derived(contextRows(app.contexts).filter((c) => !descendants.has(c.id)));
  async function rename() {
    if (!context || name.trim() === context.name || app.disabled) return;
    const old = context.name;
    renameError = '';
    try {
      await app.change({ type: 'rename_context', id: context.id, name });
      previous = old;
      name = name.trim();
    } catch (cause) {
      renameError = errorMessage(cause);
    }
  }
  async function setting(change: Change) {
    settingsError = '';
    try {
      await app.change(change);
    } catch (cause) {
      settingsError = errorMessage(cause);
    }
  }
  async function show() {
    color = context?.color ?? '';
    open = true;
    await tick();
    panel.focus();
  }
  function close() {
    open = false;
    settingsButton.focus();
  }
  async function undo(event: KeyboardEvent) {
    if (
      !context ||
      !previous ||
      !event.ctrlKey ||
      event.shiftKey ||
      event.key.toLowerCase() !== 'z' ||
      app.disabled ||
      (event.target as HTMLElement).closest('input,textarea,dialog')
    )
      return;
    event.preventDefault();
    renameError = '';
    try {
      await app.change({ type: 'rename_context', id: context.id, name: previous });
      name = previous;
      previous = null;
    } catch (cause) {
      renameError = errorMessage(cause);
    }
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if ((event.target as HTMLElement).closest('dialog')) return;
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      close();
    } else void undo(event);
  }}
/>
<!-- Primary: editable identity and totals; secondary: breadcrumb and Settings.
     Shared 12/24px type and 4/16/24px spacing group content (§2.1, §3.6).
     The color accent identifies the context without competing with Start (§8.2). -->
<section
  class="context-overview"
  aria-label="Context details"
  style={colorStyle(contextColor(context?.id ?? null, app.contexts))}
>
  <div class="overview-layout">
    <div class="identity">
      <p
        class="field-note"
        title={context?.parentId ? contextPath(app.contexts, context.parentId) : undefined}
      >
        {context?.parentId
          ? contextPath(app.contexts, context.parentId)
          : context
            ? 'Context'
            : 'Without a context'}
      </p>
      {#if context}<h1>
          <span class="sr-only">{context.name}</span><input
            class="context-title"
            aria-label="Context name"
            title={context.name}
            bind:value={name}
            disabled={app.disabled}
            onblur={rename}
            onkeydown={(event) => {
              if (event.key === 'Enter') event.currentTarget.blur();
              if (event.key === 'Escape') {
                name = context?.name ?? '';
                event.currentTarget.blur();
              }
            }}
          />
        </h1>{:else}<h1>Unassigned</h1>{/if}
    </div>
    <div class="context-stats">
      <div><span>Today</span><strong>{duration(stats.todayMs)}</strong></div>
      <div><span>Selected week</span><strong>{duration(stats.weekMs)}</strong></div>
      <div><span>Lifetime</span><strong>{duration(stats.lifetimeMs)}</strong></div>
    </div>
    {#if context}<button
        class="quiet"
        bind:this={settingsButton}
        aria-expanded={open}
        aria-controls="context-settings"
        onclick={() => (open ? close() : show())}>Settings</button
      >{/if}
    {#if renameError}<p class="error rename-error" role="alert">{renameError}</p>{/if}
  </div>
</section>
{#if context && open}
  <aside
    class="settings-panel"
    id="context-settings"
    aria-label="Context settings"
    tabindex="-1"
    bind:this={panel}
  >
    <div class="dialog-heading">
      <h2>Context settings</h2>
      <button class="icon-button" aria-label="Close settings" onclick={close}>×</button>
    </div>
    <label
      >Parent context<select
        aria-label="Parent context"
        value={context.parentId ?? ''}
        disabled={app.disabled}
        onchange={async (event) => {
          const select = event.currentTarget;
          await setting({
            type: 'move_context',
            id: context!.id,
            parentId: select.value || null,
            position: 0,
          });
          select.value = context?.parentId ?? '';
        }}
        ><option value="">No parent</option>{#each parents as parent}<option value={parent.id}
            >{parent.path}</option
          >{/each}</select
      ></label
    >
    <ColorField bind:value={color} disabled={app.disabled} />
    <div class="color-actions">
      <button
        disabled={app.disabled || !normalizeColor(color) || normalizeColor(color) === context.color}
        onclick={() => setting({ type: 'color_context', id: context!.id, color })}
        >Apply color</button
      ><button class="quiet" disabled={app.disabled} onclick={() => (color = context?.color ?? '')}
        >Reset</button
      >
    </div>
    {#if context.parentId}<button
        class="quiet parent-color"
        disabled={app.disabled}
        onclick={() =>
          (color = app.contexts.find((c) => c.id === context?.parentId)?.color ?? color)}
        >Use parent color</button
      >{/if}
    <div class="lock-section">
      <label class="checkbox"
        ><input
          type="checkbox"
          checked={context.trackingLocked}
          disabled={app.disabled ||
            (!context.trackingLocked && app.running?.contextId === context.id)}
          onchange={(event) =>
            setting({ type: 'lock_context', id: context!.id, locked: event.currentTarget.checked })}
        />Lock tracking</label
      >
      <p class="field-note">
        {app.running?.contextId === context.id
          ? 'Stop this timer before locking.'
          : 'Children remain independently trackable. Completed entries can still be corrected.'}
      </p>
    </div>
    {#if settingsError}<p class="error" role="alert">{settingsError}</p>{/if}
    <button class="quiet danger-text delete-context" disabled={app.disabled} onclick={ondelete}
      >Delete context</button
    >
  </aside>
{/if}

<style>
  .context-overview {
    container-type: inline-size;
    padding: var(--space-4) var(--space-6) var(--space-6);
  }
  .overview-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--space-6);
  }
  .identity {
    min-width: 0;
    padding-left: var(--space-4);
    border-left: var(--space-1) solid var(--context-indicator);
  }
  .identity .field-note {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-bottom: var(--space-1);
  }
  h1 {
    min-width: 0;
  }
  .context-title {
    font: inherit;
    letter-spacing: inherit;
    width: 100%;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 0;
    text-overflow: ellipsis;
  }
  .context-title:hover {
    border-bottom-color: var(--border);
  }
  .context-stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-6);
  }
  .context-stats > div {
    display: grid;
    gap: 4px;
  }
  .context-stats span {
    font-size: 12px;
    color: var(--muted);
  }
  .context-stats strong {
    font-size: 24px;
    font-weight: 400;
  }
  .rename-error {
    grid-column: 1 / -1;
  }
  .settings-panel {
    position: absolute;
    right: 24px;
    top: 16px;
    bottom: 24px;
    width: 344px;
    z-index: 12;
    background: var(--surface);
    box-shadow: var(--shadow-popover);
    border-radius: var(--radius-large);
    padding: 24px;
    overflow: auto;
  }
  .color-actions {
    display: flex;
    gap: 8px;
    margin: 12px 0 0;
  }
  .parent-color {
    margin-top: 8px;
  }
  .lock-section {
    margin-top: 24px;
    padding-top: 24px;
    border-top: 1px solid var(--border);
  }
  .checkbox {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  .delete-context {
    margin-top: 24px;
  }
  @container (max-width: 720px) {
    .overview-layout {
      grid-template-columns: minmax(0, 1fr) auto;
      gap: var(--space-4) var(--space-6);
    }
    .context-stats {
      grid-column: 1 / -1;
      grid-row: 2;
    }
  }
  @media (max-width: 1120px) {
    .context-overview {
      padding-inline: var(--space-4);
    }
  }
</style>
