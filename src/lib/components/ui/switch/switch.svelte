<script lang="ts">
  import { Switch as SwitchPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  let {
    ref = $bindable(null),
    checked = $bindable(false),
    class: className,
    ...restProps
  }: Omit<SwitchPrimitive.RootProps, "children" | "child"> = $props();
</script>

<SwitchPrimitive.Root
  bind:ref
  bind:checked
  class={cn(
    "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full p-0.5 transition-colors outline-none",
    "focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50",
    "data-[state=checked]:bg-primary data-[state=unchecked]:bg-input",
    className
  )}
  {...restProps}
>
  <SwitchPrimitive.Thumb
    class="pointer-events-none block size-5 rounded-full shadow-sm transition-transform data-[state=checked]:translate-x-5 data-[state=checked]:bg-primary-foreground data-[state=unchecked]:translate-x-0 data-[state=unchecked]:bg-foreground"
  />
</SwitchPrimitive.Root>
