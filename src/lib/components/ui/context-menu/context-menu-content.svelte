<script lang="ts">
  import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  let { ref = $bindable(null), class: className, ...restProps }: ContextMenuPrimitive.ContentProps = $props();
</script>

<ContextMenuPrimitive.Portal>
  <ContextMenuPrimitive.Content
    bind:ref
    class={cn(
      "glass z-50 min-w-48 rounded-2xl p-1.5 text-popover-foreground outline-none",
      "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
      className
    )}
    {...restProps}
  />
</ContextMenuPrimitive.Portal>
