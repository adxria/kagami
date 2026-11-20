<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import * as transport from '$transport';
  import type { Controller } from '../lib/controller.svelte';
  import { breakDuration, validBreakPercentage } from '../lib/focus';
  import { HOUR, MINUTE } from '../lib/time';
  import { errorMessage, type DataPreview, type ImportMode } from '../lib/types';
  import Modal from './Modal.svelte';

  let { app }: { app: Controller } = $props();
  let preview = $state<DataPreview | null>(null);
  let mode = $state<ImportMode | null>(null);
  let operation = $state<'export' | 'choose' | 'import' | null>(null);
  let hasRecovery = $state(false);
  let status = $state('');
  let error = $state('');
  let importError = $state('');
  let percentage = $state<number | undefined>(untrack(() => app.breakPercentage));
  let preferenceStatus = $state('');
  let preferenceError = $state('');
  let mounted = false;
  const disabled = $derived(app.disabled || operation !== null);
  const hourlyBreak = $derived(
    validBreakPercentage(percentage) ? describeDuration(breakDuration(HOUR, percentage)) : '',
  );

  function describeDuration(ms: number) {
    const minutes = Math.floor(ms / MINUTE);
    const seconds = (ms % MINUTE) / 1_000;
    const parts = [];
    if (minutes || !seconds) parts.push(`${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`);
    if (seconds) parts.push(`${seconds} ${seconds === 1 ? 'second' : 'seconds'}`);
    return parts.join(' ');
  }

  function saveBreakPercentage(event: SubmitEvent) {
    event.preventDefault();
    preferenceStatus = preferenceError = '';
    if (!validBreakPercentage(percentage)) return;
    try {
      app.setBreakPercentage(percentage);
      preferenceStatus = 'Break percentage saved.';
    } catch (cause) {
      preferenceError = `Could not save the break percentage. ${errorMessage(cause)}`;
    }
  }

  onMount(() => {
    mounted = true;
    if (app.available) {
      void transport.hasRecoveryFiles().then(
        (exists) => {
          hasRecovery ||= exists;
        },
        (cause) => {
          error = errorMessage(cause);
        },
      );
    }
    return () => {
      mounted = false;
    };
  });

  async function exportData() {
    if (disabled) return;
    operation = 'export';
    status = error = '';
    try {
      if (await transport.exportData()) status = 'Data exported.';
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      operation = null;
    }
  }

  async function choose(recovery = false) {
    if (disabled) return;
    operation = 'choose';
    status = error = '';
    try {
      const selected = await transport.chooseImport(recovery);
      if (selected && !mounted) {
        await transport.cancelImport(selected.token);
        return;
      }
      mode = null;
      importError = '';
      preview = selected;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      operation = null;
    }
  }

  function cancel() {
    if (!preview || operation === 'import') return;
    const { token } = preview;
    preview = null;
    void transport.cancelImport(token).catch((cause) => (error = errorMessage(cause)));
  }

  async function applyImport() {
    if (!preview || !mode || disabled || (mode === 'replace' && app.running)) return;
    const { token, recordings } = preview;
    const replacing = mode === 'replace';
    operation = 'import';
    importError = '';
    try {
      const recoverySaved = await app.importData(token, mode);
      hasRecovery ||= recoverySaved;
      preview = null;
      status = replacing
        ? recoverySaved
          ? 'Workspace replaced. Recovery copy saved.'
          : 'Workspace replaced.'
        : `Added ${recordings.toLocaleString()} ${recordings === 1 ? 'recording' : 'recordings'}.`;
    } catch (cause) {
      importError = errorMessage(cause);
    } finally {
      operation = null;
    }
  }
</script>

<!-- Primary: export. Import and recovery have progressively quieter actions.
     Existing surfaces and spacing tokens group the actions and their descriptions. -->
<div class="settings-content">
  <section aria-labelledby="data-heading">
    <h2 id="data-heading">Data</h2>
    <div class="data-actions">
      <div class="data-row">
        <p>Save your contexts and recordings to a JSON file.</p>
        <button class="primary" {disabled} onclick={exportData}>Export data</button>
      </div>
      <div class="data-row">
        <p>Add data from a Kagami file or replace this workspace.</p>
        <button {disabled} onclick={() => choose()}>Import data</button>
      </div>
      {#if hasRecovery}
        <div class="data-row">
          <p>Choose a copy saved before a workspace replacement.</p>
          <button class="quiet" {disabled} onclick={() => choose(true)}>Choose recovery copy</button
          >
        </div>
      {/if}
    </div>
    {#if operation === 'export' || operation === 'choose'}
      <p class="field-note" role="status">
        {operation === 'export' ? 'Exporting data…' : 'Opening data file…'}
      </p>
    {/if}
    {#if status}<p class="field-note" role="status">{status}</p>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </section>
  <section aria-labelledby="focus-settings-heading">
    <h2 id="focus-settings-heading">Focus Mode</h2>
    <form class="break-preference" onsubmit={saveBreakPercentage}>
      <label>
        Break percentage
        <input
          type="number"
          min="0"
          max="100"
          step="1"
          required
          bind:value={percentage}
          aria-describedby="break-percentage-preview"
          oninput={() => (preferenceStatus = preferenceError = '')}
        />
      </label>
      <button type="submit" disabled={!hourlyBreak || percentage === app.breakPercentage}
        >Save break percentage</button
      >
      <p class="field-note" id="break-percentage-preview">
        {hourlyBreak
          ? `1 hour of focus earns ${hourlyBreak} of break.`
          : 'Enter a whole percentage from 0 to 100.'}
      </p>
    </form>
    {#if preferenceStatus}<p class="field-note" role="status">{preferenceStatus}</p>{/if}
    {#if preferenceError}<p class="error" role="alert">{preferenceError}</p>{/if}
  </section>
</div>

{#if preview}
  <Modal title="Import data" busy={operation === 'import'} onclose={cancel}>
    <div class="file-summary">
      <p class="file-name">{preview.name}</p>
      <p class="field-note">
        {preview.contexts.toLocaleString()}
        {preview.contexts === 1 ? 'context' : 'contexts'} ·
        {preview.recordings.toLocaleString()}
        {preview.recordings === 1 ? 'recording' : 'recordings'}
      </p>
    </div>
    <fieldset class="import-modes" disabled={operation === 'import'}>
      <legend class="sr-only">Import mode</legend>
      <label>
        <input type="radio" name="import-mode" value="add" bind:group={mode} />
        <span>
          <strong>Add to current workspace</strong>
          <span class="field-note"
            >Copies every recording. Importing this file again adds another copy.</span
          >
        </span>
      </label>
      <label>
        <input type="radio" name="import-mode" value="replace" bind:group={mode} />
        <span>
          <strong>Replace workspace</strong>
          <span class="field-note"
            >Replaces your contexts and recordings. If you have data, Kagami saves a recovery copy
            first.</span
          >
        </span>
      </label>
    </fieldset>
    {#if mode === 'replace' && app.running}
      <p class="field-note">Stop the current timer before replacing the workspace.</p>
    {/if}
    {#if operation === 'import'}<p class="field-note" role="status">Importing data…</p>{/if}
    {#if importError}<p class="error" role="alert">{importError}</p>{/if}
    <div class="dialog-actions">
      <button disabled={operation === 'import'} onclick={cancel}>Cancel</button>
      <button
        class:primary={mode !== 'replace'}
        class:danger={mode === 'replace'}
        disabled={disabled || !mode || (mode === 'replace' && !!app.running)}
        onclick={applyImport}>{mode === 'replace' ? 'Replace workspace' : 'Add data'}</button
      >
    </div>
  </Modal>
{/if}

<style>
  .settings-content {
    flex: 1;
    overflow: auto;
    padding: var(--space-4) var(--space-6) var(--space-8);
  }
  section {
    display: grid;
    gap: var(--space-4);
    max-width: 40rem;
  }
  section + section {
    margin-top: var(--space-8);
  }
  .break-preference {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-2) var(--space-3);
  }
  .break-preference label {
    margin: 0;
  }
  .break-preference input {
    width: 7rem;
    min-height: 39px;
  }
  .break-preference p {
    flex: 0 0 100%;
  }
  .data-actions {
    display: grid;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
  }
  .data-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-3);
    padding: var(--space-4);
  }
  .data-row + .data-row {
    border-top: 1px solid var(--border-subtle);
  }
  .data-row p {
    color: var(--text-secondary);
  }
  .file-name {
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .file-summary {
    display: grid;
    gap: var(--space-1);
  }
  .import-modes {
    display: grid;
    gap: var(--space-2);
    border: 0;
    padding: 0;
    margin: var(--space-4) 0;
  }
  .import-modes label {
    display: flex;
    flex-direction: row;
    align-items: flex-start;
    gap: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-3);
    margin: 0;
    cursor: pointer;
  }
  .import-modes label:has(:checked) {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .import-modes label span {
    display: block;
  }
  .import-modes strong {
    font-size: 14px;
    color: var(--text);
  }
  .import-modes input {
    flex: none;
    width: auto;
    margin: var(--space-1) 0 0;
    accent-color: var(--accent);
  }
</style>
