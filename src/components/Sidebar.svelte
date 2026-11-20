<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import type { Controller } from '../lib/controller.svelte';
  import { contextRows, colorStyle, subtreeIds } from '../lib/contexts';
  import Icon from './Icon.svelte';
  import { duration } from '../lib/time';
  import { errorMessage, type Context } from '../lib/types';
  let {
    app,
    oncreate,
    onfocusmode,
  }: {
    app: Controller;
    oncreate: (parentId: string | null) => void;
    onfocusmode: () => void;
  } = $props();
  function savedWidth() {
    try {
      return Math.max(200, Math.min(400, Number(localStorage.getItem('kagami.sidebar')) || 244));
    } catch {
      return 244;
    }
  }
  let width = $state(untrack(savedWidth)),
    dragging = $state<string | null>(null),
    drop = $state<Destination | null>(null),
    failure = $state<{ targetId: string | null; message: string } | null>(null);
  type Destination = {
    targetId: string | null;
    placement: 'before' | 'after' | 'inside' | 'root';
    parentId: string | null;
    position: number;
    label: string;
  };
  let list: HTMLDivElement;
  let scrollFrame = 0,
    scrollSpeed = 0,
    suppressClick = false;
  const forbidden = $derived(dragging ? subtreeIds(app.contexts, dragging) : new Set<string>());
  const rows = $derived(contextRows(app.contexts));
  function stopScroll() {
    cancelAnimationFrame(scrollFrame);
    scrollFrame = 0;
    scrollSpeed = 0;
  }
  function clearDrag() {
    dragging = null;
    drop = null;
    stopScroll();
    // A drag must not activate the link or add-child button underneath its release.
    setTimeout(() => {
      suppressClick = false;
    }, 0);
  }
  onDestroy(stopScroll);
  function edgeScroll(event: DragEvent) {
    if (!dragging) return;
    const rect = list.getBoundingClientRect();
    if (
      event.clientX < rect.left ||
      event.clientX > rect.right ||
      event.clientY < rect.top ||
      event.clientY > rect.bottom
    ) {
      stopScroll();
      return;
    }
    const edge = 32;
    scrollSpeed =
      event.clientY < rect.top + edge
        ? -Math.ceil((rect.top + edge - event.clientY) / 4)
        : event.clientY > rect.bottom - edge
          ? Math.ceil((event.clientY - rect.bottom + edge) / 4)
          : 0;
    if (!scrollSpeed) {
      stopScroll();
      return;
    }
    if (!scrollFrame) {
      const scroll = () => {
        list.scrollTop += scrollSpeed;
        scrollFrame = requestAnimationFrame(scroll);
      };
      scrollFrame = requestAnimationFrame(scroll);
    }
  }
  function destination(event: DragEvent, target: Context | null): Destination | null {
    if (!dragging || app.disabled || (target && forbidden.has(target.id))) return null;
    let placement: Destination['placement'] = 'root';
    if (target) {
      const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
      const ratio = (event.clientY - rect.top) / rect.height;
      placement = ratio < 0.25 ? 'before' : ratio > 0.75 ? 'after' : 'inside';
    }
    const parentId = target ? (placement === 'inside' ? target.id : target.parentId) : null;
    const siblings = app.contexts
      .filter((c) => c.parentId === parentId && c.id !== dragging)
      .sort((a, b) => a.position - b.position);
    const position =
      target && placement !== 'inside'
        ? siblings.findIndex((c) => c.id === target.id) + (placement === 'after' ? 1 : 0)
        : siblings.length;
    const source = app.contexts.find((c) => c.id === dragging);
    if (!source || (source.parentId === parentId && source.position === position)) return null;
    const targetPath = rows.find((row) => row.id === target?.id)?.path;
    return {
      targetId: target?.id ?? null,
      placement,
      parentId,
      position,
      label: !target
        ? 'Move to end of top level'
        : placement === 'inside'
          ? `Move into ${targetPath} · at end`
          : `Move ${placement} ${targetPath}`,
    };
  }
  function dragOver(event: DragEvent, target: Context | null) {
    if (!dragging) return;
    event.preventDefault();
    drop = destination(event, target);
    if (event.dataTransfer) event.dataTransfer.dropEffect = drop ? 'move' : 'none';
  }
  function resize(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    const from = event.clientX,
      initial = width;
    const move = (e: PointerEvent) =>
      (width = Math.max(200, Math.min(400, initial + e.clientX - from)));
    const end = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', end);
      handle.removeEventListener('pointercancel', end);
      try {
        localStorage.setItem('kagami.sidebar', String(width));
      } catch {
        /* Optional preference. */
      }
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', end, { once: true });
    handle.addEventListener('pointercancel', end, { once: true });
  }
  async function move(event: DragEvent, target: Context | null) {
    if (!dragging) return;
    event.preventDefault();
    const next = destination(event, target);
    const id = dragging;
    clearDrag();
    if (!next) return;
    failure = null;
    try {
      await app.change({
        type: 'move_context',
        id,
        parentId: next.parentId,
        position: next.position,
      });
    } catch (cause) {
      failure = { targetId: next.targetId, message: errorMessage(cause) };
    }
  }
</script>

<svelte:window
  ondragend={clearDrag}
  ondrop={(event) => {
    if (dragging) event.preventDefault();
    clearDrag();
  }}
  onblur={clearDrag}
  ondragleave={(event) => {
    if (!event.relatedTarget) {
      drop = null;
      stopScroll();
    }
  }}
  ondragover={(event) => {
    edgeScroll(event);
    if (dragging && !(event.target as HTMLElement).closest('.context-row, .root-drop-area')) {
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = 'none';
      drop = null;
    }
  }}
/>

<!-- Primary: workspace selection. Secondary: context creation. Totals and drag
     affordances stay quieter using existing surface/text tokens and 12/16/24px spacing. -->
<aside
  class="sidebar"
  style:width="{width}px"
  onclickcapture={(event) => {
    if (dragging || suppressClick) {
      event.preventDefault();
      event.stopPropagation();
    }
  }}
>
  <button class="brand" aria-label="Kagami home" onclick={() => app.navigate({ kind: 'week' })}
    >かがみ <span>鏡</span></button
  >
  <button
    class="nav-row"
    class:active={app.page.kind === 'week'}
    onclick={() => app.navigate({ kind: 'week' })}
    ><Icon name="calendar" /> <span>Week</span></button
  >
  <button id="sidebar-focus-mode" class="nav-row" onclick={onfocusmode}
    ><Icon name="focus" /> <span>Focus Mode</span></button
  >
  <div class="context-toolbar">
    <h2>Contexts</h2>
    <button
      class="icon-button"
      aria-label="New context"
      disabled={app.disabled}
      onclick={() => oncreate(null)}>+</button
    >
  </div>
  <div class="context-list" bind:this={list}>
    {#each rows as context (context.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="context-row"
        class:active={app.page.kind === 'context' && app.page.id === context.id}
        class:drag-source={dragging === context.id}
        class:drop-inside={drop?.targetId === context.id && drop.placement === 'inside'}
        class:drop-before={drop?.targetId === context.id && drop.placement === 'before'}
        class:drop-after={drop?.targetId === context.id && drop.placement === 'after'}
        style={`${colorStyle(context.color)};--depth:${Math.min(context.depth, 6)}`}
        draggable={!app.disabled}
        ondragstart={(event) => {
          dragging = context.id;
          suppressClick = true;
          failure = null;
          event.dataTransfer?.setData('text/plain', context.id);
          if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
        }}
        ondragover={(event) => dragOver(event, context)}
        ondrop={(event) => move(event, context)}
      >
        <button
          class="context-link"
          title={context.path}
          onclick={() => app.navigate({ kind: 'context', id: context.id })}
          ><span class="context-dot"></span><span class="truncate">{context.name}</span
          >{#if context.trackingLocked}<span class="lock" title="Tracking locked"
              ><Icon name="lock" /><span class="sr-only">Tracking locked</span></span
            >{/if}</button
        >
        <span class="context-total"
          >{duration(app.totals({ kind: 'context', id: context.id }).weekMs)}</span
        >
        <button
          class="context-add icon-button"
          aria-label={`Add child to ${context.name}`}
          disabled={app.disabled}
          onclick={() => oncreate(context.id)}>+</button
        >
      </div>
      {#if failure?.targetId === context.id}<p class="error move-error" role="alert">
          {failure.message}
        </p>{/if}
    {/each}
    {#if !app.contexts.length}<p class="sidebar-hint">
        Create a context to organize your time.
      </p>{/if}
    {#if app.totals( { kind: 'unassigned' } ).lifetimeMs > 0 || app.running?.contextId === null}<button
        class="nav-row"
        class:active={app.page.kind === 'unassigned'}
        onclick={() => app.navigate({ kind: 'unassigned' })}
        ><span class="context-dot"></span>Unassigned</button
      >{/if}
    {#if dragging}<!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="root-drop-area"
        ondragover={(event) => dragOver(event, null)}
        ondrop={(event) => move(event, null)}
      >
        {#if app.contexts.find((context) => context.id === dragging)?.parentId}
          <div class="root-drop" class:drop-inside={drop?.placement === 'root'}>
            Move to top level
          </div>
        {/if}
      </div>
    {/if}
  </div>
  <p class="sr-only" role="status">{drop?.label ?? ''}</p>
  {#if failure && failure.targetId === null}<p class="error move-error" role="alert">
      {failure.message}
    </p>{/if}
  <div class="week-total">
    <span>Selected week</span><strong>{duration(app.totals({ kind: 'week' }).weekMs)}</strong>
  </div>
  <button
    class="nav-row settings-link"
    class:active={app.page.kind === 'settings'}
    aria-label="App settings"
    aria-current={app.page.kind === 'settings' ? 'page' : undefined}
    onclick={() => app.navigate({ kind: 'settings' })}
    ><Icon name="settings" /> <span>Settings</span></button
  >
  <!-- The WAI-ARIA window splitter is a focusable separator. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="resize-handle"
    role="separator"
    aria-label="Sidebar width"
    aria-orientation="vertical"
    aria-valuenow={width}
    aria-valuemin="200"
    aria-valuemax="400"
    tabindex="0"
    onpointerdown={resize}
    onkeydown={(event) => {
      if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
        event.preventDefault();
        width = Math.max(200, Math.min(400, width + (event.key === 'ArrowRight' ? 8 : -8)));
        try {
          localStorage.setItem('kagami.sidebar', String(width));
        } catch {}
      }
    }}
  ></div>
</aside>

<style>
  .sidebar {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 24px 12px 20px;
    background: var(--sidebar);
    border-right: 1px solid var(--border-subtle);
  }
  .brand {
    font-family: 'Noto Serif JP Brand', serif;
    font-size: 22px;
    font-weight: 400;
    line-height: 1.2;
    gap: 4px;
    align-items: baseline;
    align-self: flex-start;
    background: transparent;
    border: 0;
    padding: 4px 12px;
    margin-bottom: 48px;
  }
  .brand span {
    color: var(--accent);
  }
  .nav-row {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    justify-content: flex-start;
    gap: 12px;
    width: 100%;
    min-height: 40px;
    margin-bottom: 4px;
  }
  .nav-row.active {
    background: var(--surface);
    color: var(--text);
  }
  .context-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 24px 4px 8px 12px;
  }
  h2,
  .sidebar-hint {
    font-size: 12px;
    color: var(--muted);
    font-weight: 400;
  }
  .sidebar-hint {
    padding: 8px 12px;
  }
  .context-list {
    display: flex;
    flex-direction: column;
    overflow: auto;
    min-height: 0;
    flex: 1;
  }
  .context-list > :global(*) {
    flex-shrink: 0;
  }
  .context-row {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--radius);
    color: var(--text-secondary);
  }
  .drag-source {
    opacity: 0.45;
  }
  .lock {
    display: flex;
    color: var(--muted);
  }
  .context-row:hover {
    background: var(--surface);
  }
  .context-row.active {
    background: var(--surface-raised);
    color: var(--text);
  }
  .context-link {
    padding: 8px 4px 8px calc(12px + var(--depth) * 14px);
    border: 0;
    background: transparent;
    color: inherit;
    justify-content: flex-start;
    min-width: 0;
    flex: 1;
    min-height: 40px;
  }
  .context-link:hover {
    background: transparent;
  }
  .context-total {
    font-size: 12px;
    color: var(--muted);
    padding: 0 8px;
    pointer-events: none;
  }
  .context-add {
    position: absolute;
    right: 4px;
    opacity: 0;
    background: var(--surface);
    pointer-events: none;
  }
  .context-row:hover .context-add,
  .context-add:focus-visible {
    opacity: 1;
    pointer-events: auto;
  }
  .context-row:hover .context-total,
  .context-row:has(.context-add:focus-visible) .context-total {
    opacity: 0;
  }
  .week-total {
    padding: 20px 12px 0;
    border-top: 1px solid var(--border-subtle);
    display: grid;
    gap: 4px;
  }
  .week-total span {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .week-total strong {
    font-size: 30px;
    font-weight: 400;
    letter-spacing: -1px;
  }
  .settings-link {
    margin-top: var(--space-4);
    margin-bottom: 0;
  }
  .resize-handle {
    position: absolute;
    right: -3px;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: col-resize;
    z-index: 5;
    touch-action: none;
  }
  .resize-handle:hover,
  .resize-handle:focus {
    background: var(--accent);
  }
  .drop-inside {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
    background: var(--accent-subtle);
  }
  .drop-before::before,
  .drop-after::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
    pointer-events: none;
    z-index: 2;
  }
  .drop-before::before {
    top: 0;
  }
  .drop-after::after {
    bottom: 0;
  }
  /* Keep the hint grouped with its rows (§3.6); the remaining space accepts drops too. */
  .root-drop-area {
    flex: 1;
  }
  .root-drop {
    padding: var(--space-3);
    margin: var(--space-2) 0;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: 12px;
  }
  .move-error {
    padding: var(--space-2) var(--space-3);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
</style>
