<script lang="ts">
  import {
    dateLayout,
    dateOrder,
    segmentBounds,
    segmentWidth,
    timeSegments,
    type DateTimeParts,
    type Segment,
  } from '../lib/entry-time';
  let {
    label,
    parts = $bindable(),
    disabled,
    onedit,
  }: { label: string; parts: DateTimeParts; disabled: boolean; onedit: () => void } = $props();
  const layout = dateLayout(),
    order = [...dateOrder(layout), ...timeSegments];
  let fieldset: HTMLFieldSetElement;
  function update(segment: Segment, value: string) {
    parts = { ...parts, [segment]: value };
    onedit();
  }
  function key(event: KeyboardEvent, segment: Segment) {
    if (event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
    const i = order.indexOf(segment);
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      const next = order[i + (event.key === 'ArrowRight' ? 1 : -1)];
      if (next) {
        event.preventDefault();
        fieldset.querySelector<HTMLInputElement>(`[data-segment="${next}"]`)?.focus();
      }
    }
    if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      const [min, max] = segmentBounds(segment, parts);
      update(
        segment,
        String(
          Math.max(min, Math.min(max, Number(parts[segment]) + (event.key === 'ArrowUp' ? 1 : -1))),
        ).padStart(segmentWidth(segment), '0'),
      );
    }
  }
</script>

{#snippet input(segment: Segment)}
  {@const [min, max] = segmentBounds(segment, parts)}
  <input
    data-segment={segment}
    class:year={segment === 'year'}
    role="spinbutton"
    aria-label={`${label} ${segment}`}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuenow={parts[segment] ? Number(parts[segment]) : undefined}
    inputmode="numeric"
    autocomplete="off"
    maxlength={segmentWidth(segment)}
    placeholder={segment === 'year' ? 'YYYY' : '00'}
    value={parts[segment]}
    onfocus={(event) => event.currentTarget.select()}
    oninput={(event) => {
      update(segment, event.currentTarget.value.replace(/\D/g, '').slice(0, segmentWidth(segment)));
      event.currentTarget.value = parts[segment];
    }}
    onkeydown={(event) => key(event, segment)}
    onblur={() => {
      if (parts[segment])
        parts = { ...parts, [segment]: parts[segment].padStart(segmentWidth(segment), '0') };
    }}
  />
{/snippet}
<fieldset bind:this={fieldset} {disabled}>
  <legend>{label}</legend>
  <div class="datetime-fields">
    <div>
      <span>Date</span>
      <div class="segments">
        {#each layout as part}{#if 'segment' in part}{@render input(part.segment)}{:else}<span
              >{part.literal}</span
            >{/if}{/each}
      </div>
    </div>
    <div>
      <span>Time <span class="muted">24h</span></span>
      <div class="segments">
        {#each timeSegments as segment, i}{#if i}<span>:</span>{/if}{@render input(segment)}{/each}
      </div>
    </div>
  </div>
</fieldset>

<style>
  fieldset {
    border: 0;
    padding: 0;
    margin: 24px 0 0;
    min-width: 0;
  }
  legend {
    font-size: 12px;
    font-weight: 600;
    margin-bottom: 8px;
  }
  .datetime-fields {
    display: grid;
    grid-template-columns: 1.15fr 1fr;
    gap: 12px;
  }
  .datetime-fields > div > span {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }
  .segments {
    display: flex;
    align-items: center;
    min-height: 40px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-sunken);
    padding: 4px 8px;
    direction: ltr;
  }
  input {
    border: 0;
    border-radius: 2px;
    background: transparent;
    width: calc(2ch + 8px);
    padding: 2px 4px;
    text-align: center;
  }
  input.year {
    width: calc(4ch + 8px);
  }
  input:focus {
    outline: 2px solid var(--accent);
    outline-offset: 0;
    background: var(--accent-subtle);
  }
  input::selection {
    background: var(--accent);
    color: var(--on-accent);
  }
  .segments > span {
    color: var(--muted);
    white-space: pre;
  }
</style>
