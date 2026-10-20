<script lang="ts">
  import { Dialog as DialogPrimitive } from "bits-ui";
  import type { Snippet } from "svelte";
  import X from "@lucide/svelte/icons/x";
  import DialogOverlay from "./dialog-overlay.svelte";
  import { cn } from "$lib/utils";

  let {
    ref = $bindable(null),
    class: className,
    showCloseButton = true,
    children,
    ...restProps
  }: Omit<DialogPrimitive.ContentProps, "children"> & {
    showCloseButton?: boolean;
    children?: Snippet;
  } = $props();
</script>

<DialogPrimitive.Portal>
  <DialogOverlay />
  <DialogPrimitive.Content
    bind:ref
    class={cn(
      "glass fixed top-1/2 left-1/2 z-50 grid w-full max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 gap-5 rounded-2xl p-6 shadow-2xl outline-none sm:max-w-md",
      "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
      className
    )}
    {...restProps}
  >
    {@render children?.()}
    {#if showCloseButton}
      <DialogPrimitive.Close
        class="absolute top-4 right-4 rounded-md p-1 text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50"
      >
        <X class="size-4" />
        <span class="sr-only">Close</span>
      </DialogPrimitive.Close>
    {/if}
  </DialogPrimitive.Content>
</DialogPrimitive.Portal>
