<script lang="ts">
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Copy from "@lucide/svelte/icons/copy";
  import { toast } from "svelte-sonner";
  import * as ContextMenu from "$lib/components/ui/context-menu";
  import LiveEditor from "./live-editor.svelte";
  import { journal, type Entry } from "$lib/journal.svelte";
  import { timeLabel } from "$lib/dates";

  let { entry, ondelete }: { entry: Entry; ondelete: (id: string) => void } = $props();

  let editor = $state<{ focusStart: () => void } | undefined>();
  let el: HTMLDivElement;

  // New entries grab the caret as soon as they render.
  $effect(() => {
    if (journal.focusId === entry.id && editor) {
      journal.focusId = null;
      editor.focusStart();
    }
  });

  /* ---------- swipe right to delete (touch and pen) ---------- */
  const THRESHOLD = 96;
  let dx = $state(0);
  let dragging = $state(false);
  let startX = 0, startY = 0, pid = -1, axis: "h" | "v" | null = null, moved = false;

  function down(e: PointerEvent) {
    if (e.pointerType === "mouse" || !e.isPrimary) return;
    startX = e.clientX; startY = e.clientY; pid = e.pointerId; axis = null; moved = false;
  }
  function move(e: PointerEvent) {
    if (e.pointerId !== pid) return;
    const mx = e.clientX - startX, my = e.clientY - startY;
    if (!axis && (Math.abs(mx) > 10 || Math.abs(my) > 10)) {
      axis = mx > 0 && Math.abs(mx) > Math.abs(my) * 1.2 ? "h" : "v";
      if (axis === "h") { (e.currentTarget as HTMLElement).setPointerCapture(pid); dragging = true; }
    }
    if (axis === "h") {
      moved = true;
      // resist when the entry can't be deleted (the blank draft)
      dx = Math.max(0, entry.draft ? Math.min(mx * 0.25, 28) : mx);
    }
  }
  function up(e: PointerEvent) {
    if (e.pointerId !== pid) return;
    pid = -1;
    dragging = false;
    if (axis === "h" && !entry.draft && dx > THRESHOLD) {
      dx = el.clientWidth;
      setTimeout(() => ondelete(entry.id), 220);
    } else {
      dx = 0;
    }
  }
  function cancel(e: PointerEvent) {
    if (e.pointerId !== pid) return;
    pid = -1; dragging = false; dx = 0;
  }
  // a swipe shouldn't also count as a tap on the text underneath
  function suppressClick(e: MouseEvent) {
    if (moved) { e.stopPropagation(); e.preventDefault(); moved = false; }
  }

  function tidyOnLeave(e: FocusEvent) {
    if (!el.contains(e.relatedTarget as Node | null)) journal.tidy();
  }

  async function copy() {
    const text = (entry.title ? `# ${entry.title}\n\n` : "") + entry.body;
    try { await navigator.clipboard.writeText(text); toast("Copied as Markdown"); }
    catch { toast.error("Couldn't copy"); }
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger class="block">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      bind:this={el}
      class="swipe"
      data-entry-id={entry.id}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={cancel}
      onclickcapture={suppressClick}
      onfocusout={tidyOnLeave}
    >
      <div class="swipe-bg" style:opacity={Math.min(1, dx / 70)} aria-hidden="true">
        <Trash2 class="size-5" />
      </div>
      <div class="swipe-fg" data-dragging={dragging} style:transform={dx ? `translateX(${dx}px)` : undefined}>
        <header class="entry-head">
          <time class="entry-time" datetime={new Date(entry.created_at * 1000).toISOString()}>{timeLabel(entry.created_at)}</time>
          <input
            class="entry-title"
            aria-label="Entry title"
            placeholder="Title"
            value={entry.title}
            oninput={(e) => journal.update(entry.id, { title: e.currentTarget.value })}
            onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); editor?.focusStart(); } }}
          />
        </header>
        <div class="entry-body">
          <LiveEditor bind:this={editor} value={entry.body} onchange={(body) => journal.update(entry.id, { body })} />
        </div>
      </div>
    </div>
  </ContextMenu.Trigger>

  <ContextMenu.Content>
    <ContextMenu.Item onSelect={copy}><Copy />Copy as Markdown</ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item variant="destructive" disabled={entry.draft} onSelect={() => ondelete(entry.id)}>
      <Trash2 />Delete entry
    </ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu.Root>
