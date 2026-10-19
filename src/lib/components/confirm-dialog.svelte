<script lang="ts">
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";

  let {
    open = $bindable(false),
    title,
    description,
    confirmLabel = "Confirm",
    onconfirm,
  }: { open?: boolean; title: string; description: string; confirmLabel?: string; onconfirm: () => void } = $props();
</script>

<Dialog.Root bind:open>
  <Dialog.Content showCloseButton={false} class="sm:max-w-sm">
    <Dialog.Header>
      <Dialog.Title>{title}</Dialog.Title>
      <Dialog.Description>{description}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
      <Button variant="destructive" onclick={() => { open = false; onconfirm(); }}>{confirmLabel}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
