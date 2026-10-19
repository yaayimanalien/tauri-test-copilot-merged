<script lang="ts">
  import type { HTMLLabelAttributes } from "svelte/elements";
  import { cn, type WithElementRef } from "$lib/utils";

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLLabelAttributes, HTMLLabelElement> = $props();
</script>

<label
  bind:this={ref}
  class={cn("flex items-center gap-2 text-sm leading-none font-medium select-none", className)}
  {...restProps}
>
  {@render children?.()}
</label>
