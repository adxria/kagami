<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Controller } from './lib/controller.svelte';
  import { errorMessage, type Entry, type EntryDraft, type WorkspacePage } from './lib/types';
  import { addDays, duration } from './lib/time';
  import { colorStyle, contextColor, contextPath, subtreeIds } from './lib/contexts';
  import Sidebar from './components/Sidebar.svelte';
  import Settings from './components/Settings.svelte';
  import ContextPicker from './components/ContextPicker.svelte';
  import ContextView from './components/ContextView.svelte';
  import ContextForm from './components/ContextForm.svelte';
  import EntryEditor from './components/EntryEditor.svelte';
  import Confirm from './components/Confirm.svelte';
  import Timeline from './components/Timeline.svelte';
  import FocusMode from './components/FocusMode.svelte';
  import Icon from './components/Icon.svelte';
  const app = new Controller();
  let previousPage: WorkspacePage = { kind: 'week' };
  let focusLauncher = '';
  let contextDraft = $state<{ parentId: string | null } | null>(null),
    editor = $state<{ entry: Entry | null; initial: EntryDraft } | null>(null),
    preview = $state<EntryDraft | null>(null);
  let confirmation = $state<{
      title: string;
      message: string;
      label: string;
      run: () => Promise<unknown>;
    } | null>(null),
    timerError = $state('');
  const pageContext = $derived(
    app.contexts.find((c) => app.page.kind === 'context' && c.id === app.page.id) ?? null,
  );
  const subtree = $derived(
    app.page.kind === 'context' ? subtreeIds(app.contexts, app.page.id) : null,
  );
  const entries = $derived(
    (app.snapshot?.recordings ?? []).filter(
      (e) =>
        app.page.kind === 'week' ||
        (app.page.kind === 'unassigned'
          ? e.contextId === null
          : !!e.contextId && subtree?.has(e.contextId)),
    ),
  );
  const canCreate = $derived(
    app.page.kind !== 'settings' &&
      app.page.kind !== 'focus' &&
      !app.disabled &&
      app.week < app.now &&
      !pageContext?.trackingLocked &&
      !editor &&
      !contextDraft &&
      !confirmation,
  );
  const weekLabel = $derived(
    `${new Date(app.week).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })} – ${new Date(addDays(app.week, 6)).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })}`,
  );
  onMount(() => app.mount());
  function openFocus(launcher: string) {
    if (app.page.kind === 'focus') return;
    previousPage = app.page;
    focusLauncher = launcher;
    app.navigate({ kind: 'focus' });
  }
  async function closeFocus() {
    const destination = previousPage;
    app.navigate(
      destination.kind === 'context' && !app.contexts.some((c) => c.id === destination.id)
        ? { kind: 'week' }
        : destination,
    );
    await tick();
    document.getElementById(focusLauncher)?.focus();
  }
  async function timer() {
    timerError = '';
    try {
      await app.toggleTimer();
    } catch (cause) {
      timerError = errorMessage(cause);
    }
  }
  function edit(entry: Entry) {
    if (app.disabled) return;
    editor = {
      entry,
      initial: {
        contextId: entry.contextId,
        startedAt: entry.startedAt,
        endedAt: entry.endedAt ?? app.now,
      },
    };
  }
  function create(initial?: EntryDraft) {
    if (!canCreate) return;
    const endedAt = Math.min(app.now, addDays(app.week, 7));
    editor = {
      entry: null,
      initial: {
        ...(initial ?? { startedAt: endedAt - 30 * 60_000, endedAt }),
        contextId: app.page.kind === 'context' ? app.page.id : null,
      },
    };
  }
  function closeEditor() {
    editor = null;
    preview = null;
  }
  function deleteEntry(entry: Entry) {
    closeEditor();
    const running = entry.endedAt === null;
    confirmation = {
      title: running ? 'Discard current entry?' : 'Delete entry?',
      message: `${contextPath(app.contexts, entry.contextId)} · ${duration((entry.endedAt ?? app.now) - entry.startedAt, true)}. This entry will be permanently removed.`,
      label: running ? 'Discard' : 'Delete',
      run: () => app.change({ type: 'delete_recording', id: entry.id, expectedRunning: running }),
    };
  }
  function deleteContext() {
    if (!pageContext) return;
    const id = pageContext.id;
    confirmation = {
      title: 'Delete context and its history?',
      message: `Permanently delete ${pageContext.name}, all its child contexts and their recordings, including any running timer. This cannot be undone.`,
      label: 'Delete permanently',
      run: () => app.change({ type: 'delete_context', id }),
    };
  }
</script>

<div class="app-shell" class:focus-shell={app.page.kind === 'focus'}>
  {#if app.page.kind !== 'focus'}
    <Sidebar
      {app}
      oncreate={(parentId) => (contextDraft = { parentId })}
      onfocusmode={() => openFocus('sidebar-focus-mode')}
    />
  {/if}
  <main>
    {#if app.page.kind !== 'focus'}
      <header class="main-header">
        {#if app.page.kind === 'settings'}
          <h1>Settings</h1>
        {:else}
          <div class="week-navigation">
            <div>
              <span class="field-note">{app.page.kind === 'week' ? 'Week' : 'Selected week'}</span>
              <h1>{weekLabel}</h1>
            </div>
            <div class="week-arrows">
              <button
                class="icon-button"
                aria-label="Previous week"
                onclick={() => app.navigateWeek(-1)}>‹</button
              ><button class="quiet" onclick={() => app.today()}>Today</button><button
                class="icon-button"
                aria-label="Next week"
                onclick={() => app.navigateWeek(1)}>›</button
              >
            </div>
          </div>
        {/if}
        <div class="tracker-group">
          <div
            class="timer-controls"
            style={colorStyle(contextColor(app.trackingId, app.contexts))}
          >
            <ContextPicker
              contexts={app.contexts}
              value={app.trackingId}
              disabled={app.disabled}
              onselect={(id) => app.assign(id)}
            />
            <div class="timer-launch">
              <span class="timer-readout" aria-label="Elapsed time"
                >{app.running ? duration(app.now - app.running.startedAt, true) : '00:00:00'}</span
              >
              <button
                id="header-focus-mode"
                class="icon-button focus-launcher"
                aria-label="Focus Mode"
                title="Focus Mode"
                onclick={() => openFocus('header-focus-mode')}><Icon name="focus" /></button
              >
            </div>
            <button
              class="primary timer-button"
              class:stop={!!app.running}
              disabled={app.timerDisabled}
              aria-label={app.running ? 'Stop' : 'Start'}
              onclick={timer}
              ><svg viewBox="0 0 24 24" aria-hidden="true"
                >{#if app.running}<rect x="7" y="7" width="10" height="10" rx="1" />{:else}<path
                    d="M9 5.5 19 12 9 18.5Z"
                  />{/if}</svg
              ></button
            >
            <div class="discard-slot">
              {#if app.running}<button
                  class="icon-button danger-text"
                  aria-label="Discard"
                  title="Discard current entry"
                  disabled={app.disabled}
                  onclick={() => app.running && deleteEntry(app.running)}>×</button
                >{/if}
            </div>
          </div>
          {#if timerError}<p class="error timer-error" role="alert">{timerError}</p>{/if}
        </div>
      </header>
    {/if}
    {#if !app.available}<div class="notice">
        Open Kagami as a desktop app to track and save time.
      </div>{/if}
    {#if app.readError}<div class="notice error" role="alert">
        <span
          >{app.stale && app.snapshot
            ? 'The view needs refreshing. Any confirmed save is already stored. '
            : ''}{app.readError}</span
        ><button onclick={() => app.refresh()}>Refresh</button>
      </div>{/if}
    {#if app.desktopError}<div class="notice error" role="alert">
        {app.desktopError}<button class="quiet" onclick={() => (app.desktopError = '')}
          >Dismiss</button
        >
      </div>{/if}
    {#if app.page.kind === 'focus'}
      <FocusMode {app} onback={closeFocus} ondiscard={deleteEntry} />
    {:else if app.page.kind === 'settings'}
      <Settings {app} />
    {:else}
      {#if app.snapshot?.overlapIds.some((id) => entries.some((e) => e.id === id))}<div
          class="notice overlap-notice"
        >
          Some entries overlap. Totals include the full duration of each entry.
        </div>{/if}
      <div class="workspace">
        {#if app.page.kind !== 'week'}{#key app.page.kind === 'context' ? app.page.id : 'unassigned'}<ContextView
              {app}
              context={pageContext}
              ondelete={deleteContext}
            />{/key}{/if}

        <Timeline
          start={app.week}
          {entries}
          contexts={app.contexts}
          now={app.now}
          {canCreate}
          loading={app.loading}
          onedit={edit}
          oncreate={create}
          onadd={() => create()}
          draft={preview}
        />
      </div>
    {/if}
    {#if app.loading}<div class="loading" role="status">Loading local workspace…</div>{/if}
  </main>
</div>
{#if contextDraft}<ContextForm
    {app}
    parentId={contextDraft.parentId}
    onclose={() => (contextDraft = null)}
  />{/if}
{#if editor}<EntryEditor
    {app}
    entry={editor.entry}
    initial={editor.initial}
    onclose={closeEditor}
    ondelete={deleteEntry}
    onpreview={(draft) => (preview = draft)}
  />{/if}
{#if confirmation}<Confirm {...confirmation} onclose={() => (confirmation = null)} />{/if}
