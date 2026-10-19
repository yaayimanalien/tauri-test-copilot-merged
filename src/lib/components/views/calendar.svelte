<script lang="ts">
  import { journal } from "$lib/journal.svelte";
  import { dayKey } from "$lib/dates";

  let { onselect }: { onselect: (key: number) => void } = $props();

  const counts = $derived.by(() => {
    const m = new Map<number, number>();
    for (const e of journal.entries) {
      if (e.draft) continue;
      const k = dayKey(e.created_at);
      m.set(k, (m.get(k) ?? 0) + 1);
    }
    return m;
  });

  const today = dayKey(Date.now() / 1000);
  const now = new Date();

  const years = $derived.by(() => {
    let min = now.getFullYear();
    for (const k of counts.keys()) min = Math.min(min, new Date(k).getFullYear());
    return Array.from({ length: now.getFullYear() - min + 1 }, (_, i) => now.getFullYear() - i);
  });

  const yearTotal = (y: number) => {
    let n = 0;
    for (const [k, c] of counts) if (new Date(k).getFullYear() === y) n += c;
    return n;
  };

  /** Monday-first grid; null = padding cell. */
  function cells(year: number, month: number) {
    const lead = (new Date(year, month, 1).getDay() + 6) % 7;
    const days = new Date(year, month + 1, 0).getDate();
    return [
      ...Array<null>(lead).fill(null),
      ...Array.from({ length: days }, (_, i) => ({ n: i + 1, key: new Date(year, month, i + 1).getTime() })),
    ];
  }
  const monthName = (y: number, m: number) => new Date(y, m, 1).toLocaleDateString("en-GB", { month: "long" });
</script>

<div class="page">
  <h1 class="large-title">Calendar</h1>

  {#each years as year (year)}
    {@const current = year === now.getFullYear()}
    <section class="mb-10">
      <header class="bar sticky z-20 flex h-14 items-center justify-between border-b" style="top: var(--top-inset); margin-inline: calc(var(--page-px) * -1); padding-inline: var(--page-px);">
        <h2 class="text-[1.75rem] font-bold tracking-tight {current ? 'text-brand' : ''}">{year}</h2>
        <span class="text-sm text-muted-foreground">{yearTotal(year)} entries</span>
      </header>

      <div class="grid grid-cols-2 gap-x-6 gap-y-8 pt-6 sm:grid-cols-3">
        {#each Array.from({ length: 12 }, (_, i) => i) as m (m)}
          {@const isNow = current && m === now.getMonth()}
          <div>
            <h3 class="mb-2 text-[0.9375rem] font-semibold {isNow ? 'text-brand' : ''}">{monthName(year, m)}</h3>
            <div class="grid grid-cols-7 gap-y-0.5">
              {#each cells(year, m) as c, i (i)}
                {#if !c}
                  <span></span>
                {:else}
                  {@const has = counts.has(c.key)}
                  {@const isToday = c.key === today}
                  {@const future = c.key > today}
                  {#if has}
                    <button
                      type="button"
                      onclick={() => onselect(c.key)}
                      aria-label="{c.n} {monthName(year, m)} {year}, {counts.get(c.key)} {counts.get(c.key) === 1 ? 'entry' : 'entries'}"
                      class="grid aspect-square cursor-pointer place-items-center rounded-full text-xs font-semibold tabular-nums transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring
                             {isToday ? 'bg-brand text-white' : 'bg-brand/20 text-brand hover:bg-brand/30'}"
                    >{c.n}</button>
                  {:else}
                    <span
                      class="grid aspect-square place-items-center text-xs tabular-nums
                             {isToday ? 'font-semibold text-brand' : future ? 'text-muted-foreground/40' : 'text-muted-foreground'}"
                    >{c.n}</span>
                  {/if}
                {/if}
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/each}
</div>
