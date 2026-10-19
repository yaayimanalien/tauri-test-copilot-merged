<script lang="ts">
  import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  let {
    ref = $bindable(null),
    class: className,
    variant = "default",
    ...restProps
  }: ContextMenuPrimitive.ItemProps & { variant?: "default" | "destructive" } = $props();
</script>

<ContextMenuPrimitive.Item
  bind:ref
  class={cn(
    "relative flex cursor-default items-center gap-2.5 rounded-xl px-3 py-2 text-sm outline-none select-none",
    "data-[highlighted]:bg-foreground/10 data-[disabled]:pointer-events-none data-[disabled]:opacity-40",
    "[&_svg]:size-4 [&_svg]:shrink-0",
    variant === "destructive" ? "text-destructive data-[highlighted]:bg-destructive/15" : "[&_svg]:text-muted-foreground",
    className
  )}
  {...restProps}
/>
