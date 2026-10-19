import { toast } from "svelte-sonner";
import { dayKey } from "./dates";
import { api, errMsg, type RawEntry, type Status } from "./api";

export type Entry = {
  id: string;
  title: string;
  body: string;
  created_at: number; // unix seconds
  /** The blank entry that is always waiting at the top of today. */
  draft?: boolean;
};

const uid = () => crypto.randomUUID();
const nowTs = () => Math.floor(Date.now() / 1000);
const SAVE_DELAY = 400;
const SYNC_DELAY = 10_000;
const SYNC_MIN_GAP = 20_000;
const toEntry = (r: RawEntry): Entry => ({ id: r.id, title: r.title, body: r.body, created_at: r.created_at });

/* ---------- store ---------- */
class Journal {
  /** Newest first. */
  entries = $state<Entry[]>([]);
  /** Entry that should grab the caret as soon as it renders. */
  focusId = $state<string | null>(null);
  status = $state<Status | null>(null);
  syncing = $state(false);
  syncError = $state("");

  private saved = new Set<string>(); // ids that exist in the backend
  private timers = new Map<string, ReturnType<typeof setTimeout>>();
  private inflight = new Map<string, Promise<unknown>>();
  private syncTimer: ReturnType<typeof setTimeout> | undefined;
  private lastSyncTry = 0;
  private active = false;

  /* ---------- lifecycle ---------- */
  async refreshStatus() {
    this.status = await api.status();
    return this.status;
  }

  /** Call right after a successful unlock / setup / restore. */
  async open() {
    this.entries = (await api.list()).map(toEntry);
    this.saved = new Set(this.entries.map((e) => e.id));
    this.active = true;
    await this.refreshStatus();
    this.syncIfStale();
  }

  /** Flush pending saves, push to Drive (bounded wait), then lock the vault. */
  async close() {
    if (!this.active) return;
    await this.flush();
    clearTimeout(this.syncTimer);
    if (this.status?.driveConnected) {
      await Promise.race([this.sync(true), new Promise((r) => setTimeout(r, 4000))]);
    }
    this.active = false;
    await api.lock().catch(() => {});
    this.entries = [];
    this.saved.clear();
    this.status = await api.status().catch(() => this.status);
  }

  /* ---------- entries ---------- */
  /** Makes sure there is one blank entry at the top of today. */
  ensureDraft(): Entry {
    if (!this.entries.some((e) => e.draft)) {
      this.entries.unshift({ id: uid(), title: "", body: "", created_at: nowTs(), draft: true });
    }
    return this.entries.find((e) => e.draft)!;
  }

  /** Autosave target. A draft becomes a real entry on its first keystroke. */
  update(id: string, patch: Partial<Pick<Entry, "title" | "body">>) {
    const e = this.entries.find((x) => x.id === id);
    if (!e) return;
    Object.assign(e, patch);
    if (e.draft && (e.title.trim() || e.body.trim())) {
      e.draft = false;
      e.created_at = nowTs();
    }
    if (!e.draft) this.schedule(id);
  }

  private schedule(id: string) {
    clearTimeout(this.timers.get(id));
    this.timers.set(id, setTimeout(() => this.save(id), SAVE_DELAY));
  }

  private save(id: string): Promise<unknown> {
    clearTimeout(this.timers.get(id));
    this.timers.delete(id);
    const e = this.entries.find((x) => x.id === id);
    if (!e || e.draft) return Promise.resolve();
    const { title, body, created_at } = $state.snapshot(e);
    // chain per entry so create always lands before update
    const prev = this.inflight.get(id) ?? Promise.resolve();
    const run = prev
      .catch(() => {})
      .then(async () => {
        if (this.saved.has(id)) await api.update(id, title, body);
        else {
          await api.create(title, body, id, created_at);
          this.saved.add(id);
        }
        this.scheduleSync();
      })
      .catch((err) => toast.error("Couldn't save: " + errMsg(err)));
    this.inflight.set(id, run);
    return run;
  }

  /** Write every pending edit now. */
  async flush() {
    await Promise.all([...this.timers.keys()].map((id) => this.save(id)));
    await Promise.all([...this.inflight.values()]);
  }

  remove(id: string): Entry | null {
    const i = this.entries.findIndex((x) => x.id === id);
    if (i < 0) return null;
    const [e] = this.entries.splice(i, 1);
    clearTimeout(this.timers.get(id));
    this.timers.delete(id);
    this.deleteBackend(id);
    return $state.snapshot(e) as Entry;
  }

  private deleteBackend(id: string) {
    const prev = this.inflight.get(id) ?? Promise.resolve();
    const run = prev.catch(() => {}).then(async () => {
      if (this.saved.delete(id)) await api.remove(id);
      this.scheduleSync();
    }).catch((err) => toast.error("Couldn't delete: " + errMsg(err)));
    this.inflight.set(id, run);
  }

  /** Undo a delete: put it back in the list and in the vault with its original id and time. */
  restore(e: Entry) {
    this.entries.push(e);
    this.entries.sort((a, b) => b.created_at - a.created_at);
    this.save(e.id);
  }

  /** Drop saved entries that were emptied out. The draft is kept. */
  tidy() {
    const empty = this.entries.filter((e) => !e.draft && !e.title.trim() && !e.body.trim());
    for (const e of empty) {
      clearTimeout(this.timers.get(e.id));
      this.timers.delete(e.id);
      this.deleteBackend(e.id);
    }
    this.entries = this.entries.filter((e) => e.draft || e.title.trim() || e.body.trim());
  }

  /* ---------- Drive sync ---------- */
  scheduleSync() {
    if (!this.status?.driveConnected) return;
    clearTimeout(this.syncTimer);
    this.syncTimer = setTimeout(() => this.sync(true), SYNC_DELAY);
  }

  /** For app start, focus and polling: skip if we synced a moment ago. */
  syncIfStale() {
    if (!this.active || document.hidden) return;
    if (Date.now() - this.lastSyncTry < SYNC_MIN_GAP) return;
    this.sync(true);
  }

  async sync(silent = false) {
    if (this.syncing || !this.active || !this.status?.driveConnected) return;
    this.syncing = true;
    this.lastSyncTry = Date.now();
    clearTimeout(this.syncTimer);
    try {
      await this.flush();
      const r = await api.sync();
      this.syncError = "";
      if (this.status) this.status.lastBackup = r.at;
      const pulled = r.downloaded + r.removed;
      if (pulled) await this.reload();
      if (pulled) toast(`Synced ${pulled} ${pulled === 1 ? "change" : "changes"} from Drive`);
      else if (!silent) toast(r.uploaded ? `Synced, uploaded ${r.uploaded} ${r.uploaded === 1 ? "change" : "changes"}` : "Already up to date");
    } catch (err) {
      this.syncError = errMsg(err);
      if (!silent || /different Daylight vault/.test(this.syncError)) toast.error(this.syncError);
      else console.warn("Sync failed:", err);
    } finally {
      this.syncing = false;
    }
  }

  /** Merge backend state into the list without disturbing the draft or an entry being typed in. */
  private async reload() {
    if (!this.active) return;
    await this.flush();
    const fresh = (await api.list()).map(toEntry);
    if (!this.active) return;
    const draft = this.entries.find((e) => e.draft);
    const local = new Map(this.entries.map((e) => [e.id, e]));
    const next = fresh.map((f) => {
      const l = local.get(f.id);
      // keep the live object if the user is mid-edit on it
      return l && this.timers.has(f.id) ? l : f;
    });
    this.saved = new Set(fresh.map((e) => e.id));
    this.entries = draft ? [draft, ...next] : next;
    this.entries.sort((a, b) => (a.draft ? -1 : b.draft ? 1 : b.created_at - a.created_at));
  }
}

export const journal = new Journal();
export { dayKey };
