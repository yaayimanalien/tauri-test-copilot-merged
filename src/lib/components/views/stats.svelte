<script lang="ts">
  import Flame from "@lucide/svelte/icons/flame";
  import { journal } from "$lib/journal.svelte";
  import { computeStats } from "$lib/stats";

  const s = $derived(computeStats(journal.entries));

  const fmt = (n: number) => n.toLocaleString("en-GB");
  const level = (w: number) => (w === 0 ? 0 : w < 40 ? 1 : w < 100 ? 2 : w < 180 ? 3 : 4);
  const shade = ["transparent", "25%", "45%", "70%", "100%"];
  const maxHour = $derived(Math.max(1, ...s.hours));
  const maxMonth = $derived(Math.max(1, ...s.months.map((m) => m.words)));
  const maxWeekday = $derived(Math.max(1, ...s.weekdays));
  const days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const hh = (h: number) => String(h).padStart(2, "0") + ":00";
</script>

<div class="page">
  <h1 class="large-title">Stats</h1>

  <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
    <div class="glass rounded-2xl p-4">
      <div class="flex items-center gap-1.5 text-[0.8125rem] text-muted-foreground"><Flame class="size-3.5 text-brand" /> Streak</div>
      <div class="mt-2 text-[1.75rem] leading-none font-semibold tabular-nums">{s.currentStreak}<span class="ml-1 text-sm font-normal text-muted-foreground">days</span></div>
    </div>
    <div class="glass rounded-2xl p-4">
      <div class="text-[0.8125rem] text-muted-foreground">Entries</div>
      <div class="mt-2 text-[1.75rem] leading-none font-semibold tabular-nums">{fmt(s.entries)}</div>
    </div>
    <div class="glass rounded-2xl p-4">
      <div class="text-[0.8125rem] text-muted-foreground">Words</div>
      <div class="mt-2 text-[1.75rem] leading-none font-semibold tabular-nums">{fmt(s.words)}</div>
    </div>
    <div class="glass rounded-2xl p-4">
      <div class="text-[0.8125rem] text-muted-foreground">Per entry</div>
      <div class="mt-2 text-[1.75rem] leading-none font-semibold tabular-nums">{s.avgWords}<span class="ml-1 text-sm font-normal text-muted-foreground">words</span></div>
    </div>
  </div>

  <section class="glass mt-3 rounded-2xl p-5">
    <div class="mb-4 flex items-baseline justify-between">
      <h2 class="font-semibold">Last 20 weeks</h2>
      <span class="text-[0.8125rem] text-muted-foreground">{s.activeDays} days written · best streak {s.longestStreak}</span>
    </div>
    <div class="grid grid-flow-col grid-rows-7 gap-1" style="grid-auto-columns: 1fr;">
      {#each s.weeks as week, w (w)}
        {#each week as d (d.key)}
          <div
            class="aspect-square rounded-[0.3rem]"
            title="{new Date(d.key).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })}: {d.words} words"
            style="background: {d.future ? 'transparent' : level(d.words) === 0 ? 'var(--muted)' : 'color-mix(in oklab, var(--brand) ' + shade[level(d.words)] + ', transparent)'};"
          ></div>
        {/each}
      {/each}
    </div>
  </section>

  <section class="glass mt-3 rounded-2xl p-5">
    <div class="mb-4 flex items-baseline justify-between">
      <h2 class="font-semibold">Words per month</h2>
      <span class="text-[0.8125rem] text-muted-foreground">{s.thisMonth} entries this month</span>
    </div>
    <div class="flex h-32 items-end gap-3">
      {#each s.months as m (m.label)}
        <div class="flex flex-1 flex-col items-center gap-2">
          <div class="flex w-full flex-1 items-end">
            <div class="w-full rounded-md bg-brand" style="height: {Math.max(3, (m.words / maxMonth) * 100)}%; opacity: {m.words ? 1 : 0.2};"></div>
          </div>
          <span class="text-xs text-muted-foreground">{m.label}</span>
        </div>
      {/each}
    </div>
  </section>

  <div class="mt-3 grid gap-3 sm:grid-cols-2">
    <section class="glass rounded-2xl p-5">
      <h2 class="font-semibold">Time of day</h2>
      <p class="mb-4 text-[0.8125rem] text-muted-foreground">You write most around {hh(s.peakHour)}.</p>
      <div class="flex h-24 items-end gap-[3px]">
        {#each s.hours as n, h (h)}
          <div class="flex-1 rounded-[3px] bg-brand" style="height: {Math.max(4, (n / maxHour) * 100)}%; opacity: {n === maxHour ? 1 : n ? 0.45 : 0.12};" title="{hh(h)}: {n}"></div>
        {/each}
      </div>
      <div class="mt-2 flex justify-between text-[0.6875rem] text-muted-foreground"><span>00</span><span>06</span><span>12</span><span>18</span><span>23</span></div>
    </section>

    <section class="glass rounded-2xl p-5">
      <h2 class="font-semibold">Days of the week</h2>
      <p class="mb-4 text-[0.8125rem] text-muted-foreground">{s.busiestWeekday} is your busiest day.</p>
      <div class="space-y-1.5">
        {#each s.weekdays as n, i (i)}
          <div class="flex items-center gap-3 text-xs">
            <span class="w-7 text-muted-foreground">{days[i]}</span>
            <div class="h-2 flex-1 overflow-hidden rounded-full bg-muted">
              <div class="h-full rounded-full bg-brand" style="width: {(n / maxWeekday) * 100}%;"></div>
            </div>
            <span class="w-8 text-right tabular-nums text-muted-foreground">{n}</span>
          </div>
        {/each}
      </div>
    </section>
  </div>
</div>
