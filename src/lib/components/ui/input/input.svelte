<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";
  import { cn, type WithElementRef } from "$lib/utils";

  let {
    ref = $bindable(null),
    value = $bindable(),
    class: className,
    ...restProps
  }: WithElementRef<HTMLInputAttributes, HTMLInputElement> = $props();
</script>

<!-- text-base on phones avoids iOS zoom-on-focus -->
<input
  bind:this={ref}
  bind:value
  class={cn(
    "flex h-11 w-full min-w-0 rounded-lg border border-input bg-input/30 px-3.5 text-base transition-[color,box-shadow,border-color] outline-none sm:h-10 sm:text-sm",
    "placeholder:text-muted-foreground/70 selection:bg-primary selection:text-primary-foreground",
    "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/40",
    "disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/30",
    className
  )}
  {...restProps}
/>
