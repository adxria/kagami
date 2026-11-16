<script lang="ts">
  import { untrack } from 'svelte';
  import Modal from './Modal.svelte';
  import DateTimeFields from './DateTimeFields.svelte';
  import type { Controller } from '../lib/controller.svelte';
  import { errorMessage, type Entry, type EntryDraft } from '../lib/types';
  import { contextRows } from '../lib/contexts';
  import { duration, localCandidates, offsetLabel, resolveLocal } from '../lib/time';
  import { entryTimeParts, partsInput, partsProblem, timeSegments } from '../lib/entry-time';
  import { currentTime } from '$transport';
  let {
    app,
    entry = null,
    initial,
    onclose,
    ondelete,
    onpreview,
  }: {
    app: Controller;
    entry?: Entry | null;
    initial: EntryDraft;
    onclose: () => void;
    ondelete: (entry: Entry) => void;
    onpreview: (draft: EntryDraft | null) => void;
  } = $props();
  const original = untrack(() => entry ?? initial);
  const running = untrack(() => !!entry && entry.endedAt === null);
  let contextId = $state(original.contextId ?? '');
  let startParts = $state(entryTimeParts(original.startedAt));
  let endParts = $state(entryTimeParts(original.endedAt ?? currentTime()));
  let preferredStart = $state<number | null>(original.startedAt),
    preferredEnd = $state<number | null>(original.endedAt);
  let durationEdit = $state<string | null>(null),
    error = $state(''),
    busy = $state(false);
  const startText = $derived(partsInput(startParts)),
    endText = $derived(partsInput(endParts));
  const starts = $derived(localCandidates(startText)),
    ends = $derived(localCandidates(endText));
  const start = $derived(resolveLocal(startText, preferredStart)),
    end = $derived(resolveLocal(endText, preferredEnd));
  const rows = $derived(contextRows(app.contexts).filter((row) => !!entry || !row.trackingLocked));
  function durationMs(value: string) {
    if (!/^\d+:[0-5]\d:[0-5]\d$/.test(value)) return null;
    const [h, m, s] = value.split(':').map(Number);
    const ms = (h * 3600 + m * 60 + s) * 1000;
    return Number.isSafeInteger(ms) && ms <= 253402300799999 - (start ?? 0) ? ms : null;
  }
  function edited() {
    durationEdit = null;
    error = '';
  }
  function editDuration(value: string) {
    durationEdit = value;
    error = '';
    const ms = durationMs(value);
    if (ms !== null && start !== undefined) {
      preferredEnd = start + ms;
      endParts = entryTimeParts(preferredEnd);
    }
  }
  function problem(): string {
    const order = ['year', 'month', 'day', ...timeSegments] as const;
    const startProblem = partsProblem(startParts, [...order]);
    if (startProblem) return `Start: ${startProblem.message}`;
    if (start === undefined)
      return 'Start: this local time does not exist because of a daylight-saving change.';
    if (start < 0 || start > currentTime()) return 'Start must be in the past or present.';
    if (running) return '';
    if (durationEdit !== null && durationMs(durationEdit) === null)
      return 'Enter duration as HH:MM:SS. Hours may exceed 24.';
    const endProblem = partsProblem(endParts, [...order]);
    if (endProblem) return `End: ${endProblem.message}`;
    if (end === undefined)
      return 'End: this local time does not exist because of a daylight-saving change.';
    const unchangedZero =
      entry &&
      entry.startedAt === entry.endedAt &&
      start === entry.startedAt &&
      end === entry.endedAt;
    if (!unchangedZero && end <= start) return 'End must be later than Start.';
    if (end > currentTime()) return 'End must be in the past or present.';
    return '';
  }
  $effect(() => {
    const valid = start !== undefined && end !== undefined && end > start && end <= app.now;
    onpreview(
      !entry && valid ? { contextId: contextId || null, startedAt: start!, endedAt: end! } : null,
    );
  });
  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    error = problem();
    if (error) return;
    busy = true;
    try {
      if (entry)
        await app.change({
          type: 'edit_recording',
          id: entry.id,
          contextId: contextId || null,
          startedAt: start!,
          endedAt: running ? null : end!,
          expectedRunning: running,
        });
      else
        await app.change({
          type: 'create_recording',
          contextId: contextId || null,
          startedAt: start!,
          endedAt: end!,
        });
      onclose();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  title={entry ? (running ? 'Edit running entry' : 'Edit entry') : 'Add entry'}
  {busy}
  {onclose}
>
  <form onsubmit={save} novalidate>
    <label
      >Context<select bind:value={contextId} disabled={busy}
        ><option value="">Unassigned</option>{#each rows as row}<option
            value={row.id}
            disabled={row.trackingLocked && (!entry || running)}
            >{row.path}{row.trackingLocked ? ' · Locked' : ''}</option
          >{/each}</select
      ></label
    >
    <DateTimeFields label="Start" bind:parts={startParts} disabled={busy} onedit={edited} />
    {#if starts.length > 1}<label
        >Start UTC offset<select
          value={Math.floor((start ?? 0) / 1000) * 1000}
          disabled={busy}
          onchange={(event) => {
            preferredStart = Number(event.currentTarget.value);
            edited();
          }}
          >{#each starts as value}<option {value}>{offsetLabel(value)}</option>{/each}</select
        ></label
      >{/if}
    {#if !running}<DateTimeFields
        label="End"
        bind:parts={endParts}
        disabled={busy}
        onedit={edited}
      />
      {#if ends.length > 1}<label
          >End UTC offset<select
            value={Math.floor((end ?? 0) / 1000) * 1000}
            disabled={busy}
            onchange={(event) => {
              preferredEnd = Number(event.currentTarget.value);
              edited();
            }}
            >{#each ends as value}<option {value}>{offsetLabel(value)}</option>{/each}</select
          ></label
        >{/if}
    {/if}
    <label class="duration-field"
      >Duration<input
        aria-label="Duration"
        value={durationEdit ??
          (start === undefined ? '' : duration((running ? app.now : (end ?? start)) - start, true))}
        readonly={running}
        disabled={busy}
        placeholder="HH:MM:SS"
        oninput={(event) => editDuration(event.currentTarget.value)}
      /></label
    >
    <p class="field-note">
      {running
        ? 'Stop the timer to edit duration. Assignment changes the whole entry.'
        : 'Hours:minutes:seconds. Changing duration updates End.'}
    </p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="dialog-actions">
      {#if entry}<button
          type="button"
          class="quiet danger-text push-left"
          disabled={busy}
          onclick={() => ondelete(entry!)}>{running ? 'Discard' : 'Delete'}</button
        >{/if}<button type="button" disabled={busy} onclick={onclose}>Cancel</button><button
        class="primary"
        disabled={busy || app.stale}>Save</button
      >
    </div>
  </form>
</Modal>

<style>
  .duration-field {
    margin-top: 24px;
  }
</style>
