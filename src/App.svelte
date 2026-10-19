<script lang="ts">
  import { onMount, tick } from "svelte";
  import Plus from "@lucide/svelte/icons/plus";
  import { Toaster } from "$lib/components/ui/sonner";
  import LockScreen from "$lib/components/lock-screen.svelte";
  import Nav from "$lib/components/nav.svelte";
  import TimelineView from "$lib/components/views/timeline.svelte";
  import CalendarView from "$lib/components/views/calendar.svelte";
  import StatsView from "$lib/components/views/stats.svelte";
  import SettingsView from "$lib/components/views/settings.svelte";
  import { journal } from "$lib/journal.svelte";
  import { api } from "$lib/api";
  import type { View } from "$lib/types";

  const AUTO_LOCK_MS = 5 * 60 * 1000;
  const HIDDEN_LOCK_MS = 60 * 1000;
  const SYNC_POLL_MS = 2 * 60 * 1000;

  let screen = $state<"boot" | "lock" | "app">("boot");
  let lockMode = $state<"unlock" | "setup">("unlock");
  let lockNotice = $state("");
  let view = $state<View>("timeline");
  let dayTarget = $state<number | null>(null);
  let idleTimer: ReturnType<typeof setTimeout> | undefined;
  let hiddenAt = 0;

  onMount(() => {
    api.status().then(
      (st) => {
        lockMode = st.exists ? "unlock" : "setup";
        screen = "lock";
      },
      () => (screen = "lock"),
    );
    const poll = setInterval(() => journal.syncIfStale(), SYNC_POLL_MS);
    const evts = ["pointerdown", "keydown", "scroll", "touchstart"];
    evts.forEach((t) => window.addEventListener(t, resetIdle, { passive: true, capture: true }));
    return () => {
      clearInterval(poll);
      evts.forEach((t) => window.removeEventListener(t, resetIdle, { capture: true }));
    };
  });

  async function enter() {
    await journal.open();
    screen = "app";
    view = "timeline";
    journal.focusId = journal.ensureDraft().id; // a blank entry is ready as soon as the app opens
    window.scrollTo(0, 0);
    resetIdle();
  }

  async function lock(reason = "") {
    if (screen !== "app") return;
    clearTimeout(idleTimer);
    journal.tidy();
    await journal.close();
    lockNotice = reason;
    lockMode = "unlock";
    screen = "lock";
  }

  function resetIdle() {
    clearTimeout(idleTimer);
    if (screen === "app") idleTimer = setTimeout(() => lock("Locked after inactivity"), AUTO_LOCK_MS);
  }

  function onvisibility() {
    if (document.hidden) { hiddenAt = Date.now(); return; }
    if (screen === "app" && hiddenAt && Date.now() - hiddenAt > HIDDEN_LOCK_MS) lock();
    else journal.syncIfStale();
    hiddenAt = 0;
  }

  async function newEntry() {
    view = "timeline";
    journal.focusId = journal.ensureDraft().id;
    await tick();
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function openDay(key: number) {
    dayTarget = key;
    view = "timeline";
  }

  function onkeydown(e: KeyboardEvent) {
    if (screen !== "app" || !(e.metaKey || e.ctrlKey)) return;
    const k = e.key.toLowerCase();
    if (k === "n") { e.preventDefault(); newEntry(); }
    else if (k === "l") { e.preventDefault(); lock(); }
  }
</script>

<svelte:window {onkeydown} onfocus={() => journal.syncIfStale()} />
<svelte:document onvisibilitychange={onvisibility} />
<Toaster />

{#if screen === "boot"}
  <div class="min-h-dvh"></div>
{:else if screen === "lock"}
  <LockScreen mode={lockMode} notice={lockNotice} onunlock={enter} />
{:else}
  <div class="safe-top-scrim"></div>

  <main class="app-main">
    {#if view === "timeline"}
      <TimelineView target={dayTarget} ontarget={() => (dayTarget = null)} />
    {:else if view === "calendar"}
      <CalendarView onselect={openDay} />
    {:else if view === "stats"}
      <StatsView />
    {:else}
      <SettingsView onlock={lock} />
    {/if}
  </main>

  {#if view === "timeline"}
    <button
      type="button"
      aria-label="New entry"
      onclick={newEntry}
      class="glass fixed right-4 bottom-[calc(5.75rem+env(safe-area-inset-bottom,0px))] z-40 grid size-12 cursor-pointer place-items-center rounded-full text-brand transition-transform outline-none focus-visible:ring-2 focus-visible:ring-ring active:scale-95 md:right-6 md:bottom-6"
    >
      <Plus class="size-5" strokeWidth={2.4} />
    </button>
  {/if}

  <Nav bind:view />
{/if}
