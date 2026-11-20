import * as transport from '$transport';
import type { Change, ImportMode, Page, Query, Totals, Workspace, WorkspacePage } from './types';
import { errorMessage } from './types';
import { addDays, dayStart, weekStart } from './time';
import { readBreakPercentage, writeBreakPercentage } from './focus';

export class Controller {
  snapshot = $state<Workspace | null>(null);
  week = $state(weekStart(transport.currentTime()));
  page = $state<Page>({ kind: 'week' });
  now = $state(transport.currentTime());
  busy = $state(false);
  loading = $state(true);
  stale = $state(false);
  readError = $state('');
  desktopError = $state('');
  selectedContext = $state<string | null>(null);
  breakPercentage = $state(readBreakPercentage());
  breakDurationEdit = $state<{ id: string; committed: string; input: string | null } | null>(null);
  readonly available = transport.available;
  private zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  private monday = this.weekParts();
  private requested = 0;
  private pending: Promise<void> | null = null;
  get disabled() {
    return !this.available || !this.snapshot || this.busy || this.stale || this.loading;
  }
  get contexts() {
    return this.snapshot?.contexts ?? [];
  }
  get running() {
    return this.snapshot?.running ?? null;
  }
  get activeBreak() {
    return this.snapshot?.activeBreak ?? null;
  }
  get trackingId() {
    return this.running ? this.running.contextId : this.selectedContext;
  }
  get timerDisabled() {
    return (
      this.disabled ||
      (!this.running && !!this.contexts.find((c) => c.id === this.selectedContext)?.trackingLocked)
    );
  }
  setBreakPercentage(value: number) {
    writeBreakPercentage(value);
    this.breakPercentage = value;
  }
  query(): Query {
    this.syncZone();
    const today = dayStart(this.now);
    return {
      weekStart: this.week,
      weekEnd: addDays(this.week, 7),
      todayStart: today,
      todayEnd: addDays(today, 1),
    };
  }
  refresh(): Promise<void> {
    this.requested++;
    if (!this.available) {
      this.loading = false;
      return Promise.resolve();
    }
    if (this.pending) return this.pending;
    this.pending = this.drain().finally(() => {
      this.pending = null;
    });
    return this.pending;
  }
  private async drain() {
    let processed = 0;
    while (processed < this.requested) {
      const token = this.requested;
      const query = this.query();
      try {
        const snapshot = await transport.read(query);
        if (token === this.requested) {
          this.snapshot = snapshot;
          if (
            this.breakDurationEdit &&
            (snapshot.activeBreak?.id !== this.breakDurationEdit.id ||
              snapshot.activeBreak.endsAt !== null)
          ) {
            this.breakDurationEdit = null;
          }
          this.now = transport.currentTime();
          this.stale = false;
          this.readError = '';
          if (
            this.page.kind === 'context' &&
            !snapshot.contexts.some((c) => this.page.kind === 'context' && c.id === this.page.id)
          )
            this.page = { kind: 'week' };
          if (
            this.selectedContext &&
            !snapshot.contexts.some((c) => c.id === this.selectedContext && !c.trackingLocked)
          )
            this.selectedContext = null;
        }
      } catch (error) {
        if (token === this.requested) {
          this.stale = true;
          this.readError = errorMessage(error);
        }
      }
      processed = token;
    }
    this.loading = false;
  }
  change(change: Change) {
    return this.save(() => transport.apply(change));
  }
  importData(token: string, mode: ImportMode) {
    return this.save(() => transport.applyImport(token, mode));
  }
  private async save<T>(apply: () => Promise<T>) {
    if (this.disabled) throw new Error('Refresh the workspace before making another change.');
    this.busy = true;
    try {
      const receipt = await apply();
      // Commit succeeded. A refresh failure must never reopen a creation draft or
      // imply the write failed. Disable further writes until the snapshot recovers.
      this.stale = true;
      await this.refresh();
      return receipt;
    } finally {
      this.busy = false;
    }
  }
  navigate(page: Page) {
    this.page = page;
    if (!this.running && (page.kind === 'context' || page.kind === 'unassigned'))
      this.selectedContext = page.kind === 'context' ? page.id : null;
  }
  navigateWeek(delta: number) {
    this.syncZone();
    this.week = addDays(this.week, delta * 7);
    this.monday = this.weekParts();
    this.loading = true;
    void this.refresh();
  }
  today() {
    this.syncZone();
    this.week = weekStart(this.now);
    this.monday = this.weekParts();
    this.loading = true;
    void this.refresh();
  }
  async assign(contextId: string | null) {
    if (this.running) await this.change({ type: 'assign_timer', id: this.running.id, contextId });
    else this.selectedContext = contextId;
  }
  async toggleTimer() {
    if (this.running) {
      const { id, contextId } = this.running;
      await this.change({ type: 'stop_timer', id });
      this.selectedContext = contextId;
    } else await this.change({ type: 'start_timer', contextId: this.selectedContext });
  }
  async prepareBreak(id: string) {
    const running = this.running;
    await this.change({ type: 'prepare_break', id });
    if (running?.id === id) this.selectedContext = running.contextId;
  }
  async endBreak(id: string) {
    const activeBreak = this.activeBreak;
    await this.change({ type: 'end_break', id });
    if (activeBreak?.id === id) {
      this.selectedContext =
        this.contexts.find((c) => c.id === activeBreak.contextId && !c.trackingLocked)?.id ?? null;
    }
  }
  totals(target: Exclude<WorkspacePage, { kind: 'settings' }>): Totals {
    const snapshot = this.snapshot;
    const base =
      target.kind === 'week'
        ? snapshot?.totals
        : target.kind === 'unassigned'
          ? snapshot?.unassigned
          : snapshot?.contextTotals.find((t) => target.id === t.contextId)?.total;
    const result = { ...(base ?? { todayMs: 0, weekMs: 0, lifetimeMs: 0 }) };
    const running = snapshot?.running;
    let matches =
      !!running &&
      (target.kind === 'week' || (target.kind === 'unassigned' && running.contextId === null));
    if (running && target.kind === 'context') {
      let ancestor = running.contextId;
      while (ancestor) {
        if (ancestor === target.id) {
          matches = true;
          break;
        }
        ancestor = this.contexts.find((c) => c.id === ancestor)?.parentId ?? null;
      }
    }
    if (snapshot && running && matches) {
      const from = Math.max(snapshot.calculatedAt, running.startedAt);
      const to = Math.max(from, this.now);
      const clip = (lo: number, hi: number) => Math.max(0, Math.min(to, hi) - Math.max(from, lo));
      result.todayMs += clip(snapshot.query.todayStart, snapshot.query.todayEnd);
      result.weekMs += clip(snapshot.query.weekStart, snapshot.query.weekEnd);
      result.lifetimeMs += to - from;
    }
    return result;
  }
  mount() {
    let disposed = false;
    let unsubscribe = () => {};
    let signature = this.calendarSignature();
    let lastTick = transport.currentTime();
    void this.refresh();
    if (this.available)
      void transport
        .subscribe(
          () => {
            void this.refresh();
          },
          (error) => {
            this.desktopError = errorMessage(error);
          },
        )
        .then((cleanup) => {
          if (disposed) cleanup();
          else unsubscribe = cleanup;
        })
        .catch((error) => {
          this.desktopError = errorMessage(error);
        });
    const visible = () => {
      if (!document.hidden) {
        this.now = transport.currentTime();
        signature = this.calendarSignature();
        void this.refresh();
      }
    };
    const interval = window.setInterval(() => {
      if (document.hidden) return;
      const now = transport.currentTime();
      const resumed = now - lastTick > 5_000 || now < lastTick;
      lastTick = now;
      const activeBreak = this.activeBreak;
      const endsAt = activeBreak?.endsAt;
      if (resumed || (activeBreak?.finished === false && endsAt != null && now >= endsAt)) {
        void this.refresh();
      }
      if (this.running || endsAt != null || Math.abs(now - this.now) >= 60_000) this.now = now;
      const next = this.calendarSignature();
      if (signature !== next) {
        signature = next;
        void this.refresh();
      }
    }, 1_000);
    window.addEventListener('focus', visible);
    document.addEventListener('visibilitychange', visible);
    return () => {
      disposed = true;
      unsubscribe();
      clearInterval(interval);
      window.removeEventListener('focus', visible);
      document.removeEventListener('visibilitychange', visible);
    };
  }
  private syncZone() {
    const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (zone !== this.zone) {
      // Preserve the selected calendar Monday when its UTC offset changes.
      this.week = new Date(...this.monday).getTime();
      this.zone = zone;
    }
  }
  private weekParts(): [number, number, number] {
    const date = new Date(this.week);
    return [date.getFullYear(), date.getMonth(), date.getDate()];
  }
  private calendarSignature() {
    const now = transport.currentTime();
    return `${dayStart(now)}:${new Date(now).getTimezoneOffset()}:${Intl.DateTimeFormat().resolvedOptions().timeZone}`;
  }
}
