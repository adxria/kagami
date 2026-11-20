<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { currentTime } from '$transport';
  import type { Controller } from '../lib/controller.svelte';
  import { colorStyle, contextColor, contextPath } from '../lib/contexts';
  import { breakDuration } from '../lib/focus';
  import { focusBackground } from '../lib/focus-background';
  import { duration, MAX_TIMESTAMP, parseDuration } from '../lib/time';
  import { errorMessage, type Entry } from '../lib/types';
  import ContextPicker from './ContextPicker.svelte';
  import Icon from './Icon.svelte';

  let {
    app,
    onback,
    ondiscard,
  }: {
    app: Controller;
    onback: () => void;
    ondiscard: (entry: Entry) => void;
  } = $props();

  const headings = {
    idle: 'Ready to focus',
    focus: 'Focusing',
    setup: 'Break ready',
    break: 'On break',
    finished: 'Break finished',
  };
  let heading: HTMLHeadingElement | undefined;
  let primary = $state<HTMLButtonElement>();
  let editClock = $state<HTMLButtonElement>();
  let durationInput = $state<HTMLInputElement>();
  let actionError = $state('');
  let durationError = $state('');

  const activeBreak = $derived(app.activeBreak);
  const running = $derived(app.running);
  const endsAt = $derived(activeBreak?.endsAt ?? null);
  const breakDelta = $derived(endsAt === null ? null : endsAt - app.now);
  const phase = $derived(
    activeBreak
      ? breakDelta === null
        ? 'setup'
        : activeBreak.finished
          ? 'finished'
          : 'break'
      : running
        ? 'focus'
        : 'idle',
  );
  const edit = $derived(
    app.breakDurationEdit?.id === activeBreak?.id ? app.breakDurationEdit : null,
  );
  const proposedDuration = $derived(
    edit?.committed ??
      duration(breakDuration(activeBreak?.focusedMs ?? 0, app.breakPercentage), true),
  );
  const time = $derived(
    phase === 'setup'
      ? proposedDuration
      : duration(
          breakDelta !== null
            ? phase === 'finished'
              ? Math.max(0, -breakDelta)
              : Math.ceil(Math.max(0, breakDelta) / 1_000) * 1_000
            : running
              ? app.now - running.startedAt
              : 0,
          true,
        ),
  );
  const clockLabel = $derived(
    phase === 'break'
      ? 'Time remaining'
      : phase === 'finished'
        ? 'Time since break ended'
        : 'Elapsed focus time',
  );
  const contextId = $derived(activeBreak ? activeBreak.contextId : app.trackingId);
  const context = $derived(contextPath(app.contexts, contextId));
  const contextStyle = $derived(colorStyle(contextColor(contextId, app.contexts)));

  onMount(() => heading?.focus());

  async function act(action: () => Promise<unknown>) {
    if (app.disabled) return;
    actionError = durationError = '';
    try {
      await action();
      await tick();
      if (app.disabled) heading?.focus();
      else primary?.focus();
    } catch (cause) {
      actionError = errorMessage(cause);
    }
  }

  async function openDurationEditor(id: string) {
    durationError = '';
    app.breakDurationEdit = { id, committed: proposedDuration, input: proposedDuration };
    await tick();
    durationInput?.focus();
    durationInput?.select();
  }

  function validateDuration(value: string): number | null {
    const ms = parseDuration(value.trim());
    if (ms === null || ms <= 0 || ms > MAX_TIMESTAMP - currentTime()) {
      durationError = 'Enter a positive duration as HH:MM:SS.';
      return null;
    }
    return ms;
  }

  async function editKey(event: KeyboardEvent) {
    if (event.isComposing || !edit || edit.input === null) return;
    if (event.key !== 'Enter' && event.key !== 'Escape') return;
    event.preventDefault();
    if (event.key === 'Enter') {
      const ms = validateDuration(edit.input);
      if (ms === null) return;
      edit.committed = duration(ms, true);
    }
    durationError = '';
    edit.input = null;
    await tick();
    editClock?.focus();
  }

  function startBreak(id: string) {
    const ms = validateDuration(edit?.input ?? proposedDuration);
    if (ms === null) {
      (durationInput ?? editClock)?.focus();
      return;
    }
    void act(() => app.change({ type: 'start_break', id, durationMs: ms }));
  }
</script>

<!-- The clock leads. The heading names the state, the context carries the color,
     and a single filled action sits below the clock. Use Kagami's type and spacing. -->
<div class="focus-view" data-phase={phase} style={contextStyle}>
  <canvas class="focus-background" aria-hidden="true" use:focusBackground={contextStyle}></canvas>
  <header class="focus-navigation">
    <button class="quiet" onclick={onback}>
      <span aria-hidden="true">←</span> Back to workspace
    </button>
  </header>

  <div class="focus-stage">
    <div class="focus-content" style:--clock-units={Math.max(4.65, time.length * 0.6)}>
      <div class="focus-reading">
        <h1 bind:this={heading} tabindex="-1" aria-live="polite">{headings[phase]}</h1>
        {#if activeBreak && phase === 'setup'}
          {#if edit && edit.input !== null}
            <div class="clock-editor">
              <input
                class="clock"
                type="text"
                aria-label="Break duration"
                aria-describedby={durationError ? 'break-duration-error' : 'break-duration-format'}
                aria-invalid={durationError ? true : undefined}
                autocomplete="off"
                spellcheck="false"
                bind:this={durationInput}
                bind:value={edit.input}
                oninput={() => (durationError = '')}
                onkeydown={editKey}
                disabled={app.disabled}
              />
              <p id="break-duration-format" class="clock-caption">Hours, minutes, seconds</p>
            </div>
          {:else}
            <button
              type="button"
              class="clock-button"
              aria-label={'Edit break duration, ' + time}
              aria-describedby={durationError ? 'break-duration-error' : undefined}
              bind:this={editClock}
              disabled={app.disabled}
              onclick={() => openDurationEditor(activeBreak.id)}
            >
              <span class="clock">{time}</span>
              <span class="clock-caption edit-caption">Edit duration</span>
            </button>
          {/if}
          {#if durationError}
            <p class="error" id="break-duration-error" role="alert">{durationError}</p>
          {/if}
        {:else}
          <div class="clock" role="timer" aria-label={clockLabel}>{time}</div>
          {#if activeBreak}
            <p class="clock-caption">{clockLabel}</p>
          {/if}
        {/if}
      </div>

      <div class="focus-controls">
        {#if running || activeBreak}
          <div class="focus-context">
            <p class="context-name">
              <span class="context-dot" aria-hidden="true"></span>
              <span class="truncate" title={context}>{context}</span>
            </p>
            {#if phase === 'setup' && activeBreak}
              <p class="field-note">{duration(activeBreak.focusedMs)} of focus saved</p>
            {/if}
          </div>
        {:else}
          <ContextPicker
            contexts={app.contexts}
            value={app.trackingId}
            disabled={app.disabled}
            onselect={(id) => app.assign(id)}
          />
        {/if}

        <div class="focus-actions">
          {#if activeBreak && phase === 'setup'}
            <button
              class="primary"
              bind:this={primary}
              disabled={app.disabled}
              onclick={() => startBreak(activeBreak.id)}><Icon name="play" /> Start break</button
            >
            <button
              class="quiet"
              disabled={app.disabled}
              onclick={() => act(() => app.endBreak(activeBreak.id))}>Dismiss break</button
            >
          {:else if activeBreak}
            <button
              class="primary"
              bind:this={primary}
              disabled={app.disabled}
              onclick={() => act(() => app.endBreak(activeBreak.id))}
              >{phase === 'finished' ? 'Done' : 'End break'}</button
            >
          {:else if running}
            <button
              class="primary"
              bind:this={primary}
              disabled={app.disabled}
              onclick={() => act(() => app.prepareBreak(running.id))}>Take a break</button
            >
            <button
              class="quiet"
              disabled={app.disabled}
              onclick={() => act(() => app.toggleTimer())}>Save and stop</button
            >
          {:else}
            <button
              class="primary"
              bind:this={primary}
              disabled={app.timerDisabled}
              onclick={() => act(() => app.toggleTimer())}><Icon name="play" /> Start focus</button
            >
          {/if}
        </div>
        {#if running}
          <button class="quiet discard" disabled={app.disabled} onclick={() => ondiscard(running)}
            >Discard recording</button
          >
        {/if}
        {#if actionError}<p class="error" role="alert">{actionError}</p>{/if}
      </div>
    </div>
  </div>
</div>

<style>
  .focus-view {
    --focus-accent: var(--accent-hover);
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    position: relative;
    isolation: isolate;
    background-color: var(--surface-sunken);
    /* Approximate the canvas glow until the dithered background is drawn. */
    background-image: radial-gradient(
      circle farthest-corner at top left,
      color-mix(in srgb, var(--context-indicator) 62%, transparent),
      color-mix(in srgb, var(--context-indicator) 46%, transparent) 9%,
      color-mix(in srgb, var(--context-indicator) 34.5%, transparent) 15%,
      color-mix(in srgb, var(--context-indicator) 27%, transparent) 20%,
      color-mix(in srgb, var(--context-indicator) 20%, transparent) 26%,
      color-mix(in srgb, var(--context-indicator) 12%, transparent) 41%,
      color-mix(in srgb, var(--context-indicator) 4.5%, transparent) 68%,
      color-mix(in srgb, var(--context-indicator) 0.8%, transparent) 85%,
      transparent 90%
    );
  }
  .focus-background {
    position: absolute;
    inset: 0;
    z-index: -1;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .focus-view[data-phase='setup'],
  .focus-view[data-phase='break'] {
    --focus-accent: var(--warning-text);
  }
  .focus-view[data-phase='finished'] {
    --focus-accent: #a9f2dc;
  }
  .focus-navigation {
    display: flex;
    justify-content: flex-end;
    padding: var(--space-6) var(--space-8);
  }
  .focus-navigation button,
  .focus-actions .quiet,
  .discard {
    color: var(--text-secondary);
  }
  .focus-stage {
    --focus-lift: var(--space-6);
    flex: 1;
    display: grid;
    place-items: center;
    overflow: auto;
    padding: var(--space-6) var(--space-6) calc(var(--space-8) + 2 * var(--focus-lift));
  }
  @media (max-height: 700px) {
    .focus-stage {
      --focus-lift: var(--space-3);
    }
  }
  .focus-content {
    container-type: inline-size;
    display: grid;
    gap: var(--space-8);
    width: min(100%, 32rem);
  }
  .focus-reading {
    display: grid;
    gap: var(--space-1);
    justify-items: start;
    min-width: 0;
  }
  h1 {
    color: var(--text-secondary);
    font-size: 18px;
    letter-spacing: 0;
  }
  @media (max-width: 600px) {
    h1 {
      color: var(--text);
    }
  }
  h1:focus {
    outline: none;
  }
  .clock {
    max-width: 100%;
    color: color-mix(in srgb, var(--text) 78%, var(--focus-accent));
    font-size: min(7rem, calc(100cqi / var(--clock-units)));
    font-weight: 400;
    line-height: 1.08;
    letter-spacing: -0.055em;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .clock-caption {
    color: var(--text-secondary);
    font-size: 13px;
  }
  .clock-button,
  .clock-editor {
    display: grid;
    justify-items: start;
    gap: var(--space-1);
    min-width: 0;
  }
  .clock-button {
    justify-content: start;
    text-align: left;
  }
  .clock-button,
  .clock-button:hover:not(:disabled) {
    padding: 0;
    background: transparent;
    border: 0;
  }
  .edit-caption {
    text-decoration: underline;
    text-underline-offset: var(--space-1);
  }
  .clock-button:hover:not(:disabled) .edit-caption {
    color: var(--text);
  }
  .clock-editor {
    width: 100%;
  }
  .clock-editor input {
    width: 100%;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: var(--radius-entry);
  }
  .focus-controls {
    display: grid;
    justify-items: start;
    gap: var(--space-4);
    min-width: 0;
  }
  .focus-context {
    display: grid;
    gap: var(--space-1);
    min-width: 0;
    max-width: 100%;
  }
  .context-name {
    display: flex;
    align-items: center;
    min-width: 0;
    gap: var(--space-2);
    color: var(--text-secondary);
  }
  .focus-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .focus-actions button {
    min-height: 42px;
    padding-inline: var(--space-4);
  }
  .focus-actions .primary {
    color: var(--on-accent);
    background: var(--focus-accent);
    border-color: transparent;
  }
  .focus-actions .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--focus-accent), var(--text) 25%);
    border-color: transparent;
  }
  .discard {
    padding: var(--space-1) 0;
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: var(--space-1);
  }
  .discard:hover:not(:disabled) {
    color: var(--danger-text);
    background: transparent;
    border-color: transparent;
  }
</style>
