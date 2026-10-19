<script lang="ts">
  import BookOpen from "@lucide/svelte/icons/book-open";
  import CalendarDays from "@lucide/svelte/icons/calendar-days";
  import Activity from "@lucide/svelte/icons/activity";
  import Settings from "@lucide/svelte/icons/settings";
  import type { View } from "$lib/types";

  let { view = $bindable() }: { view: View } = $props();

  const items = [
    { id: "timeline", label: "Timeline", icon: BookOpen },
    { id: "calendar", label: "Calendar", icon: CalendarDays },
    { id: "stats", label: "Stats", icon: Activity },
    { id: "settings", label: "Settings", icon: Settings },
  ] as const;
</script>

<!--
  Phones: floating tab bar at the bottom, label under each icon.
  ≥ md:  vertical island on the left, icons only.
  ≥ lg:  same island with titles to the right of the icons.
-->
<nav
  aria-label="Main"
  class="glass fixed z-50 flex flex-row gap-1 rounded-[1.75rem] p-1.5
         inset-x-0 bottom-[max(0.75rem,env(safe-area-inset-bottom))] mx-auto w-fit
         md:inset-x-auto md:bottom-auto md:left-4 md:top-1/2 md:mx-0 md:-translate-y-1/2 md:flex-col md:rounded-[1.6rem] md:p-2"
>
  {#each items as item (item.id)}
    {@const active = view === item.id}
    <button
      type="button"
      aria-current={active ? "page" : undefined}
      aria-label={item.label}
      onclick={() => (view = item.id)}
      class="relative flex min-w-[4.25rem] cursor-pointer flex-col items-center gap-0.5 rounded-[1.35rem] px-3 py-2 text-[0.625rem] font-medium transition-colors outline-none
             focus-visible:ring-2 focus-visible:ring-ring
             md:min-w-0 md:flex-row md:gap-3 md:p-3 md:text-sm lg:py-2.5 lg:pr-5 lg:pl-3.5
             {active ? 'bg-foreground/10 text-foreground' : 'text-muted-foreground hover:text-foreground'}"
    >
      <item.icon class="size-[1.375rem] shrink-0 transition-colors {active ? 'text-brand' : ''}" strokeWidth={active ? 2.2 : 1.8} />
      <span class="md:sr-only lg:not-sr-only">{item.label}</span>
    </button>
  {/each}
</nav>
