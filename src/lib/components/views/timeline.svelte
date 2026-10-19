<script lang="ts">
  import { tick, untrack } from "svelte";
  import { toast } from "svelte-sonner";
  import { Button } from "$lib/components/ui/button";
  import Entry from "$lib/components/entry.svelte";
  import { journal, type Entry as EntryT } from "$lib/journal.svelte";
  import { dayKey, dayParts, relativeDay } from "$lib/dates";

  let { target = null, ontarget }: { target?: number | null; ontarget?: () => void } = $props();

  let visibleDays = $state(30);

  // Newest day first. Today lists newest entry first (the blank draft on top);
  // every other day reads top to bottom, oldest first.
  const groups = $derived.by(() => {
    const todayKey = dayKey(Date.now() / 1000);
    const map = new Map<number, EntryT[]>();
    for (const e of journal.entries) {
      const k = dayKey(e.created_at);
      const list = map.get(k);
      if (list) list.push(e); else map.set(k, [e]);
    }
    if (!map.has(todayKey)) map.set(todayKey, []);
    return [...map.keys()]
      .sort((a, b) => b - a)
      .map((key) => ({ key, items: key === todayKey ? map.get(key)! : [...map.get(key)!].reverse() }));
  });

  const shown = $derived(groups.slice(0, visibleDays));

  // jump to a day chosen in the calendar
  $effect(() => {
    const t = target;
    if (t == null) return;
    untrack(() => {
      const idx = groups.findIndex((g) => g.key === t);
      if (idx >= visibleDays) visibleDays = idx + 1;
      tick().then(() => {
        document.getElementById("day-" + t)?.scrollIntoView({ block: "start" });
        ontarget?.();
      });
    });
  });

  function remove(id: string) {
    const removed = journal.remove(id);
    if (!removed) return;
    toast("Entry deleted", { action: { label: "Undo", onClick: () => journal.restore(removed) } });
  }
</script>

<div class="page pt-1">
  {#each shown as g (g.key)}
    {@const d = dayParts(g.key)}
    {@const isToday = relativeDay(g.key) === "Today"}
    <section id={"day-" + g.key} class="day">
      <header class="day-head bar">
        <div class="flex items-center gap-3">
          <span class="day-num" data-today={isToday}>{d.num}</span>
          <div class="leading-tight">
            <div class="text-[0.9375rem] font-semibold">{d.weekday}</div>
            <div class="text-[0.8125rem] text-muted-foreground">{d.month}</div>
          </div>
        </div>
        <span class="text-sm font-medium {isToday ? 'text-brand' : 'text-muted-foreground'}">{relativeDay(g.key)}</span>
      </header>

      {#each g.items as e (e.id)}
        <Entry entry={e} ondelete={remove} />
      {/each}
    </section>
  {/each}

  {#if groups.length > visibleDays}
    <div class="flex justify-center pt-8">
      <Button variant="ghost" onclick={() => (visibleDays += 30)}>Show earlier days</Button>
    </div>
  {/if}
</div>
