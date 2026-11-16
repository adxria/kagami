<script lang="ts">
  import { tick } from 'svelte';
  import type { Context } from '../lib/types';
  import { errorMessage } from '../lib/types';
  import { colorStyle, contextColor, contextPath, contextRows } from '../lib/contexts';
  import Icon from './Icon.svelte';
  let {
    contexts,
    value,
    disabled,
    onselect,
  }: {
    contexts: Context[];
    value: string | null;
    disabled: boolean;
    onselect: (id: string | null) => Promise<void>;
  } = $props();
  const id = $props.id();
  let open = $state(false),
    search = $state(''),
    filtering = $state(false),
    active = $state(-1),
    error = $state(''),
    busy = $state(false);
  let root: HTMLDivElement;
  let input = $state<HTMLInputElement>(null!);
  const display = $derived(contextPath(contexts, value));
  const selectedLocked = $derived(contexts.find((c) => c.id === value)?.trackingLocked);
  const query = $derived(search.trim().toLowerCase());
  const options = $derived(
    [
      { id: '', name: 'Unassigned', path: 'Unassigned', trackingLocked: false },
      ...contextRows(contexts),
    ].filter((row) =>
      row.trackingLocked
        ? filtering && (row.name.toLowerCase() === query || row.path.toLowerCase() === query)
        : !filtering || row.path.toLowerCase().includes(query),
    ),
  );
  const selectable = $derived(
    options.flatMap((option, index) => (option.trackingLocked ? [] : [index])),
  );
  async function reveal() {
    await tick();
    document.getElementById(`${id}-option-${active}`)?.scrollIntoView({ block: 'nearest' });
  }
  function show() {
    if (disabled || busy || open) return;
    search = display;
    filtering = false;
    active = options.findIndex((option) => option.id === (value ?? '') && !option.trackingLocked);
    if (active < 0) active = selectable[0] ?? -1;
    error = '';
    open = true;
    input.select();
    void reveal();
  }
  function close() {
    open = false;
    search = display;
    error = '';
  }
  async function select(next: string) {
    if (disabled || busy || !options.some((o) => o.id === next && !o.trackingLocked)) return;
    busy = true;
    error = '';
    try {
      await onselect(next || null);
      close();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
  function key(event: KeyboardEvent) {
    if (event.isComposing || busy) return;
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      event.stopPropagation();
      close();
    } else if (event.key === 'Tab' && open) {
      if (!selectable.length) {
        close();
        return;
      }
      event.preventDefault();
      const index = selectable.indexOf(active);
      active =
        selectable[(index + (event.shiftKey ? -1 : 1) + selectable.length) % selectable.length];
      void reveal();
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) {
        show();
        return;
      }
      const index = selectable.indexOf(active);
      active =
        selectable[
          Math.max(0, Math.min(selectable.length - 1, index + (event.key === 'ArrowDown' ? 1 : -1)))
        ] ?? -1;
      void reveal();
    } else if (event.key === 'Enter' && open) {
      event.preventDefault();
      if (options[active]) void select(options[active].id);
    }
  }
</script>

<svelte:window
  onpointerdown={(event) => {
    if (open && !root.contains(event.target as Node)) close();
  }}
/>

<!-- Primary: editable current path and matching paths. Secondary: status and key
     hints. Shared surfaces, 12/14px type and 4/8/12px spacing (§2.1, §3.6). -->
<div
  class="picker"
  bind:this={root}
  style={colorStyle(contextColor(value, contexts))}
  onfocusout={(event) => {
    if (!root.contains(event.relatedTarget as Node | null)) close();
  }}
>
  <div class="picker-field">
    <span class="context-dot" aria-hidden="true"></span>
    <input
      bind:this={input}
      role="combobox"
      aria-label="Search contexts"
      aria-expanded={open}
      aria-autocomplete="list"
      aria-controls={open ? `${id}-options` : undefined}
      aria-activedescendant={open && active >= 0 ? `${id}-option-${active}` : undefined}
      aria-describedby={selectedLocked ? `${id}-locked` : undefined}
      value={open ? search : display}
      title={display}
      disabled={disabled && !busy}
      readonly={busy}
      autocomplete="off"
      onfocus={show}
      onclick={() => {
        show();
        input.select();
      }}
      oninput={(event) => {
        search = event.currentTarget.value;
        filtering = true;
        open = true;
        error = '';
        active = selectable[0] ?? -1;
        void reveal();
      }}
      onkeydown={key}
    />
    {#if selectedLocked}<span id={`${id}-locked`} class="field-lock" title="Tracking locked"
        ><Icon name="lock" /><span class="sr-only">Tracking locked</span></span
      >{/if}
    <span class="chevron"><Icon name="chevron" /></span>
  </div>
  {#if open}<div class="picker-popup">
      <div id={`${id}-options`} role="listbox" aria-label="Contexts" aria-busy={busy}>
        {#each options as option, index (option.id)}<button
            id={`${id}-option-${index}`}
            role="option"
            aria-selected={option.id === (value ?? '')}
            aria-disabled={option.trackingLocked}
            tabindex="-1"
            class:highlighted={index === active}
            disabled={option.trackingLocked || disabled || busy}
            onpointerdown={(event) => event.preventDefault()}
            onclick={() => select(option.id)}
          >
            <span
              class="context-dot"
              style={colorStyle(contextColor(option.id || null, contexts))}
              aria-hidden="true"
            ></span>
            <span class="option-copy"
              ><span class="option-path">{option.path}</span>{#if option.trackingLocked}<span
                  class="locked-note"
                  ><Icon name="lock" />Tracking locked — choose another context.</span
                >{/if}</span
            >
            {#if option.id === (value ?? '')}<span class="selection-mark"
                ><Icon name="check" /></span
              >{/if}
          </button>{:else}<p class="empty muted">No matching contexts</p>{/each}
      </div>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <div class="picker-hint" aria-hidden="true">
        <span><kbd>Tab</kbd> cycle</span><span><kbd>↵</kbd> select</span><span
          ><kbd>Esc</kbd> close</span
        >
      </div>
    </div>{/if}
</div>

<style>
  .picker {
    position: relative;
    min-width: 0;
    width: 340px;
  }
  .picker-field {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius);
    background: var(--surface-sunken);
  }
  .picker-field:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  input {
    width: 100%;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    text-overflow: ellipsis;
    outline: none;
  }
  input:focus-visible {
    outline: none;
  }
  .chevron,
  .field-lock {
    display: flex;
    color: var(--muted);
    pointer-events: none;
  }
  .picker-popup {
    position: absolute;
    right: 0;
    top: calc(100% + var(--space-2));
    width: min(400px, calc(100vw - 48px));
    padding: var(--space-2);
    z-index: 20;
    border-radius: var(--radius);
    background: var(--surface-raised);
    box-shadow: var(--shadow-popover);
  }
  [role='listbox'] {
    max-height: min(320px, 50vh);
    overflow: auto;
  }
  [role='option'] {
    display: flex;
    width: 100%;
    justify-content: flex-start;
    text-align: left;
    white-space: normal;
    border: 0;
    background: transparent;
    padding: var(--space-2) var(--space-3);
  }
  [role='option']:disabled {
    opacity: 1;
    color: var(--muted);
  }
  [role='option'].highlighted {
    background: var(--surface-hover);
  }
  .option-copy {
    min-width: 0;
    flex: 1;
    display: grid;
    gap: var(--space-1);
  }
  .option-path {
    overflow-wrap: anywhere;
  }
  .locked-note,
  .picker-hint {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 12px;
    color: var(--muted);
  }
  .selection-mark {
    color: var(--accent);
    display: flex;
  }
  .picker-hint {
    justify-content: space-between;
    border-top: 1px solid var(--border);
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-3) 0;
  }
  kbd {
    font: inherit;
    color: var(--text-secondary);
  }
  .error,
  .empty {
    padding: var(--space-2) var(--space-3);
  }
  @media (max-width: 1120px) {
    .picker {
      width: 280px;
    }
  }
</style>
