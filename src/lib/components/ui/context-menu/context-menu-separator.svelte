<script lang="ts">
  import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  let { ref = $bindable(null), class: className, ...restProps }: ContextMenuPrimitive.SeparatorProps = $props();
</script>

<ContextMenuPrimitive.Separator bind:ref class={cn("-mx-1.5 my-1.5 h-px bg-border", className)} {...restProps} />
