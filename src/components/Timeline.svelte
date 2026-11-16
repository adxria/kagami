<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Modal from './Modal.svelte';
  import type { Context, Entry, EntryDraft } from '../lib/types';
  import { colorStyle, contextColor, contextPath } from '../lib/contexts';
  import {
    addDays,
    clock,
    dateLabel,
    dayStart,
    duration,
    HOUR,
    MINUTE,
    offsetLabel,
  } from '../lib/time';
  import { layoutDay, draftAt, zoomAt } from '../lib/timeline';
  let {
    start,
    entries,
    contexts,
    now,
    canCreate,
    loading,
    onedit,
    oncreate,
    onadd,
    draft,
  }: {
    start: number;
    entries: Entry[];
    contexts: Context[];
    now: number;
    canCreate: boolean;
    loading: boolean;
    onedit: (entry: Entry) => void;
    oncreate: (draft: EntryDraft) => void;
    onadd: () => void;
    draft: EntryDraft | null;
  } = $props();
  let zoom = $state(1),
    group = $state<Entry[] | null>(null);
  let scroller: HTMLDivElement;
  const days = $derived(
    Array.from({ length: 7 }, (_, i) => {
      const day = addDays(start, i),
        end = addDays(day, 1);
      return { start: day, end, ...layoutDay(entries, day, end, now, zoom) };
    }),
  );
  const maxMinutes = $derived(Math.max(...days.map((d) => (d.end - d.start) / MINUTE)));
  function title(entry: Entry) {
    return `${contextPath(contexts, entry.contextId)}\n${new Date(entry.startedAt).toLocaleString()} – ${entry.endedAt === null ? 'Running' : new Date(entry.endedAt).toLocaleString()}\n${duration((entry.endedAt ?? now) - entry.startedAt, true)}`;
  }
  async function changeZoom(next: number, anchor = scroller.clientHeight / 2) {
    const clamped = Math.max(0.5, Math.min(60, next));
    const scroll = ((scroller.scrollTop + anchor) / zoom) * clamped - anchor;
    zoom = clamped;
    await tick();
    scroller.scrollTop = scroll;
  }
  function create(event: MouseEvent, day: number) {
    if (!canCreate || (event.target as HTMLElement).closest('button')) return;
    const y = event.clientY - (event.currentTarget as HTMLElement).getBoundingClientRect().top;
    const initial = draftAt(day, y, zoom, now);
    if (initial) oncreate(initial);
  }
  onMount(() => {
    scroller.scrollTop = Math.max(
      0,
      (new Date(now).getHours() * 60 + new Date(now).getMinutes() - 180) * zoom,
    );
    const wheel = (event: WheelEvent) => {
      if (!event.altKey) return;
      event.preventDefault();
      const anchor = event.clientY - scroller.getBoundingClientRect().top;
      const delta =
        event.deltaY *
        (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? scroller.clientHeight : 1);
      void changeZoom(zoomAt(zoom, delta, scroller.scrollTop, anchor).zoom, anchor);
    };
    scroller.addEventListener('wheel', wheel, { passive: false });
    return () => scroller.removeEventListener('wheel', wheel);
  });
</script>

<!-- Primary: recorded intervals. Secondary: hour/day labels. Small markers have
     their own rail so enlarged hit targets do not suggest overlapping durations. -->
<div class="calendar" aria-label="Weekly timeline">
  <div class="calendar-head">
    <div></div>
    {#each days as day}<div class="day-heading" class:today={day.start === dayStart(now)}>
        <span>{new Date(day.start).toLocaleDateString('en-GB', { weekday: 'short' })}</span><strong
          >{new Date(day.start).getDate()}</strong
        >
      </div>{/each}
  </div>
  {#if !loading && !entries.length && !draft}<div class="empty-state">
      <strong>No time recorded this week</strong><span>Start tracking or add an entry.</span>
    </div>{/if}
  <!-- Keyboard users can focus and scroll this region. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="calendar-scroll"
    bind:this={scroller}
    tabindex="0"
    role="region"
    aria-label="Timeline. Alt plus scroll to zoom"
  >
    <div class="calendar-grid" style:height="{maxMinutes * zoom}px">
      <div class="time-gutter">
        {#each Array.from({ length: Math.ceil(maxMinutes / 60) }, (_, i) => i) as hour}<span
            style:top="{hour * 60 * zoom}px">{String(hour % 24).padStart(2, '0')}:00</span
          >{/each}
      </div>
      {#each days as day}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="day-column"
          ondblclick={(event) => create(event, day.start)}
          style:height="{((day.end - day.start) / MINUTE) * zoom}px"
        >
          {#each Array.from({ length: Math.ceil((day.end - day.start) / HOUR) }, (_, i) => day.start + i * HOUR) as instant}<div
              class="hour-line"
              style:top="{((instant - day.start) / MINUTE) * zoom}px"
            >
              {#if day.end - day.start !== 24 * HOUR}<span class="dst-label"
                  >{clock(instant)} {offsetLabel(instant)}</span
                >{/if}
            </div>{/each}
          <div class="entry-lanes" style:right={day.groups.length ? '26px' : '3px'}>
            {#each day.segments.filter((s) => !s.tiny) as segment (segment.entry.id)}
              <button
                class="calendar-entry"
                class:running={segment.entry.endedAt === null}
                style={`${colorStyle(contextColor(segment.entry.contextId, contexts))};top:${segment.top}px;height:${segment.height}px;left:${(segment.lane / segment.lanes) * 100}%;width:calc(${100 / segment.lanes}% - 2px)`}
                title={title(segment.entry)}
                aria-label={title(segment.entry)}
                onclick={() => onedit(segment.entry)}
                ><strong>{contextPath(contexts, segment.entry.contextId)}</strong
                >{#if segment.height > 40}<span
                    >{clock(segment.start)} – {segment.entry.endedAt === null
                      ? 'Running'
                      : clock(segment.end)}</span
                  >{/if}{#if segment.height > 64}<span>{duration(segment.end - segment.start)}</span
                  >{/if}</button
              >
            {/each}
          </div>
          {#each day.groups as marker, i}<button
              class="short-marker"
              style:top="{marker.top}px"
              style:height="{marker.height}px"
              title={marker.segments.length === 1
                ? title(marker.segments[0].entry)
                : `${marker.segments.length} short entries`}
              aria-label={`${dateLabel(day.start)} short entries ${i + 1}: ${marker.segments.length}`}
              onclick={() => {
                const items = marker.segments.map((s) => s.entry);
                if (items.length === 1) onedit(items[0]);
                else group = items;
              }}>{marker.segments.length > 1 ? marker.segments.length : '·'}</button
            >{/each}
          {#if draft && draft.startedAt < day.end && draft.endedAt > day.start}<div
              class="draft-preview"
              style={`${colorStyle(contextColor(draft.contextId, contexts))};top:${((Math.max(draft.startedAt, day.start) - day.start) / MINUTE) * zoom}px;height:${Math.max(16, ((Math.min(draft.endedAt, day.end) - Math.max(draft.startedAt, day.start)) / MINUTE) * zoom)}px`}
            >
              Draft
            </div>{/if}
          {#if now >= day.start && now < day.end}<div
              class="now-line"
              style:top="{((now - day.start) / MINUTE) * zoom}px"
            >
              <span></span>
            </div>{/if}
        </div>
      {/each}
    </div>
  </div>
  <div class="zoom-controls">
    <button class="quiet add-entry" disabled={!canCreate} onclick={onadd}>+ Add entry</button><span
      >Alt + scroll to zoom</span
    ><button
      class="icon-button"
      aria-label="Zoom out"
      disabled={zoom <= 0.5}
      onclick={() => changeZoom(zoom / 1.5)}>−</button
    ><button class="quiet zoom-label" aria-label="Reset zoom" onclick={() => changeZoom(1)}
      >{Math.round(zoom * 100)}%</button
    ><button
      class="icon-button"
      aria-label="Zoom in"
      disabled={zoom >= 60}
      onclick={() => changeZoom(zoom * 1.5)}>+</button
    >
  </div>
</div>
{#if group}<Modal title="Short entries" onclose={() => (group = null)}
    ><div class="group-list">
      {#each group as entry}<button
          style={colorStyle(contextColor(entry.contextId, contexts))}
          onclick={() => {
            group = null;
            onedit(entry);
          }}
          ><span class="context-dot"></span><span
            >{contextPath(contexts, entry.contextId)}<small
              >{clock(entry.startedAt, true)} · {duration(
                (entry.endedAt ?? now) - entry.startedAt,
                true,
              )}</small
            ></span
          ></button
        >{/each}
    </div></Modal
  >{/if}

<style>
  .calendar {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    border-top: 1px solid var(--border-subtle);
  }
  .calendar-head,
  .calendar-grid {
    display: grid;
    grid-template-columns: 64px repeat(7, minmax(0, 1fr));
  }
  .calendar-head {
    padding-right: 14px;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .day-heading {
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 12px 0;
    color: var(--muted);
    font-size: 12px;
  }
  .day-heading strong {
    font-size: 18px;
    font-weight: 400;
    color: var(--text-secondary);
  }
  .day-heading.today strong {
    background: var(--accent);
    color: var(--on-accent);
    border-radius: 50%;
    width: 30px;
    height: 30px;
    text-align: center;
    line-height: 30px;
  }
  .day-heading.today {
    color: var(--accent);
  }
  .calendar-scroll {
    overflow-y: scroll;
    flex: 1;
    min-height: 0;
    scrollbar-gutter: stable;
  }
  .calendar-grid {
    position: relative;
  }
  .time-gutter {
    position: relative;
  }
  .time-gutter > span {
    position: absolute;
    right: 12px;
    transform: translateY(-50%);
    font-size: 12px;
    color: var(--muted);
  }
  .day-column {
    position: relative;
    border-left: 1px solid var(--border-subtle);
  }
  .hour-line {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid var(--border-subtle);
    pointer-events: none;
  }
  .dst-label {
    position: absolute;
    right: 4px;
    font-size: 10px;
    color: var(--muted);
  }
  .entry-lanes {
    position: absolute;
    left: 3px;
    top: 0;
    bottom: 0;
    pointer-events: none;
  }
  .calendar-entry {
    position: absolute;
    display: block;
    overflow: hidden;
    min-width: 0;
    border: 0;
    border-left: 3px solid var(--context-indicator);
    border-radius: var(--radius-entry);
    background: var(--context-fill);
    color: var(--context-on-fill);
    padding: 4px 8px;
    text-align: left;
    white-space: normal;
    font-size: 12px;
    pointer-events: auto;
    line-height: 1.4;
  }
  .calendar-entry:hover {
    background: var(--context-fill);
    filter: brightness(1.13);
  }
  .calendar-entry strong {
    font-weight: 600;
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .calendar-entry > span {
    display: block;
  }
  .calendar-entry.running {
    border-top: 1px dashed var(--context-indicator);
  }
  .short-marker {
    position: absolute;
    right: 3px;
    width: 20px;
    min-height: 16px;
    padding: 0;
    border-radius: 4px;
    font-size: 12px;
    background: var(--surface-raised);
    z-index: 2;
  }
  .draft-preview {
    position: absolute;
    left: 3px;
    right: 3px;
    pointer-events: none;
    border: 1px dashed var(--context-indicator);
    background: var(--context-fill);
    color: var(--context-on-fill);
    padding: 4px 8px;
    overflow: hidden;
    font-size: 12px;
  }
  .now-line {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid var(--accent);
    pointer-events: none;
    z-index: 3;
  }
  .now-line > span {
    position: absolute;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    top: -3px;
    left: -3px;
  }
  .empty-state {
    position: absolute;
    top: 100px;
    left: 50%;
    transform: translateX(-50%);
    display: grid;
    gap: 4px;
    text-align: center;
    color: var(--muted);
    padding: 16px 24px;
    border-radius: var(--radius);
    background: var(--background);
    z-index: 4;
    pointer-events: none;
    white-space: nowrap;
  }
  .empty-state strong {
    font-weight: 400;
    color: var(--text-secondary);
  }
  .empty-state span {
    font-size: 12px;
  }
  .zoom-controls {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
    padding: 4px 16px;
    color: var(--muted);
    border-top: 1px solid var(--border-subtle);
  }
  .add-entry {
    margin-right: auto;
    font-size: 12px;
    padding: 4px 8px;
  }
  .zoom-controls > span {
    font-size: 12px;
    margin-right: 12px;
  }
  .zoom-label {
    font-size: 12px;
    min-width: 60px;
    padding: 4px;
  }
  .group-list {
    display: grid;
    gap: 8px;
    max-height: 400px;
    overflow: auto;
  }
  .group-list button {
    justify-content: flex-start;
    text-align: left;
  }
  .group-list small {
    display: block;
  }
</style>
