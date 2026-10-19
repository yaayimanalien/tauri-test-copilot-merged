<script lang="ts">
  /*
   * Obsidian-style live preview.
   * The body is split into blocks (one per line; a fenced code block is one
   * block). Inactive blocks render as formatted markdown. The block holding
   * the caret becomes a textarea showing its raw source with dimmed markers.
   */
  import { tick, untrack } from "svelte";
  import { Markdown } from "$lib/markdown";
  import { renderBlock } from "$lib/live-render";

  let {
    value,
    placeholder = "Write something…",
    onchange,
  }: { value: string; placeholder?: string; onchange?: (v: string) => void } = $props();

  type Block = { start: number; end: number; text: string; fence: boolean };

  const LIST = /^(\s*)([-*+]|(\d+)\.)\s+(.*)$/;
  const QUOTE = /^(>\s?)(.*)$/;

  let lines = $state<string[]>(untrack(() => value.split("\n")));
  let activeLine = $state<number | null>(null);
  let container: HTMLDivElement;

  const blocks = $derived.by(() => {
    const out: Block[] = [];
    let i = 0;
    while (i < lines.length) {
      if (/^```/.test(lines[i])) {
        let j = i + 1;
        while (j < lines.length && !/^```\s*$/.test(lines[j])) j++;
        const end = Math.min(j, lines.length - 1);
        out.push({ start: i, end, text: lines.slice(i, end + 1).join("\n"), fence: true });
        i = end + 1;
      } else {
        out.push({ start: i, end: i, text: lines[i], fence: false });
        i++;
      }
    }
    return out;
  });

  const activeStart = $derived(
    activeLine === null ? -1 : (blocks.find((b) => activeLine! >= b.start && activeLine! <= b.end)?.start ?? -1)
  );
  const isEmpty = $derived(lines.length === 1 && lines[0] === "");

  const cls = (b: Block) => {
    if (b.fence) return "fence";
    const h = b.text.match(/^(#{1,6})\s/);
    return h ? "h" + h[1].length : "";
  };

  const blockAt = (line: number) => blocks.find((b) => line >= b.start && line <= b.end);
  const commit = () => onchange?.(lines.join("\n"));

  /** Activate the block containing `line` and place the caret. */
  async function setActive(line: number, col: number | "end" = "end") {
    line = Math.max(0, Math.min(line, lines.length - 1));
    activeLine = line;
    await tick();
    const ta = container?.querySelector("textarea");
    const b = blockAt(line);
    if (!ta || !b) return;
    ta.focus();
    let off = ta.value.length;
    if (col !== "end") {
      off = 0;
      for (let i = b.start; i < line; i++) off += lines[i].length + 1;
      off += Math.min(col, lines[line].length);
    }
    ta.setSelectionRange(off, off);
  }

  export function focusStart() { return setActive(0, 0); }
  export function focusEnd() { return setActive(lines.length - 1, "end"); }

  /* ---------- events ---------- */
  function onInput(b: Block, e: Event) {
    const ta = e.currentTarget as HTMLTextAreaElement;
    const next = ta.value.split("\n");
    lines.splice(b.start, b.end - b.start + 1, ...next);
    commit();
    // pasted several lines into a single-line block: caret goes to the last one
    if (!b.fence && next.length > 1) setActive(b.start + next.length - 1, "end");
  }

  function onBeforeInput(b: Block, e: InputEvent) {
    const ta = e.currentTarget as HTMLTextAreaElement;
    const s = ta.selectionStart, en = ta.selectionEnd;
    const text = b.text;

    // Enter: split the line, continuing lists and quotes
    if ((e.inputType === "insertParagraph" || e.inputType === "insertLineBreak") && !b.fence) {
      e.preventDefault();
      const before = text.slice(0, s), after = text.slice(en);
      let prefix = "";
      const lm = text.match(LIST);
      const qm = !lm && text.match(QUOTE);
      if (lm) {
        const markerEnd = text.length - lm[4].length;
        if (s >= markerEnd) {
          if (!lm[4].trim() && s === text.length) { // empty item: end the list
            lines[b.start] = "";
            commit();
            setActive(b.start, 0);
            return;
          }
          prefix = lm[1] + (lm[3] ? +lm[3] + 1 + "." : lm[2]) + " ";
        }
      } else if (qm && s >= qm[1].length) {
        if (!qm[2].trim() && s === text.length) {
          lines[b.start] = "";
          commit();
          setActive(b.start, 0);
          return;
        }
        prefix = "> ";
      }
      lines.splice(b.start, 1, before, prefix + after);
      commit();
      setActive(b.start + 1, prefix.length);
      return;
    }

    // Backspace
    if (e.inputType === "deleteContentBackward" && s === en) {
      const lm = !b.fence && text.match(LIST);
      if (lm && s > 0 && s === text.length - lm[4].length) { // remove the list marker
        e.preventDefault();
        lines[b.start] = lm[1] + lm[4];
        commit();
        setActive(b.start, lm[1].length);
        return;
      }
      if (s === 0 && b.start > 0) { // join with the line above
        e.preventDefault();
        const prev = blockAt(b.start - 1)!;
        const prevLen = lines[b.start - 1].length;
        if (prev.fence) {
          if (!lines[b.start].length) { lines.splice(b.start, 1); commit(); }
        } else {
          lines[b.start - 1] += lines[b.start];
          lines.splice(b.start, 1);
          commit();
        }
        setActive(b.start - 1, prevLen);
        return;
      }
    }

    // Delete at the end of a line: pull the next line up
    if (e.inputType === "deleteContentForward" && s === en && s === ta.value.length && !b.fence && b.end < lines.length - 1) {
      e.preventDefault();
      const len = lines[b.end].length;
      lines[b.end] += lines[b.end + 1];
      lines.splice(b.end + 1, 1);
      commit();
      setActive(b.end, len);
    }
  }

  /** Is the caret on the first / last visual row of the block? (measured on the backdrop) */
  function visualRow(offset: number) {
    const bd = container.querySelector(".md-backdrop");
    if (!bd) return { first: true, last: true };
    const walker = document.createTreeWalker(bd, NodeFilter.SHOW_TEXT);
    let n = 0, node: Node | null;
    while ((node = walker.nextNode())) {
      const len = node.nodeValue!.length;
      if (offset <= n + len) {
        const r = document.createRange();
        r.setStart(node, offset - n);
        r.collapse(true);
        const rect = r.getClientRects()[0];
        if (!rect) break;
        const box = bd.getBoundingClientRect();
        const lh = parseFloat(getComputedStyle(bd).lineHeight) || 24;
        return { first: rect.top - box.top < lh * 0.9, last: box.bottom - rect.bottom < lh * 0.9 };
      }
      n += len;
    }
    return { first: true, last: true };
  }

  function onKey(b: Block, e: KeyboardEvent) {
    const ta = e.currentTarget as HTMLTextAreaElement;
    if (e.key === "Escape") { activeLine = null; ta.blur(); return; }
    if (e.isComposing || e.shiftKey || e.metaKey || e.ctrlKey || e.altKey) return;
    const s = ta.selectionStart;
    if (s !== ta.selectionEnd) return;
    const col = Math.max(0, s - (ta.value.lastIndexOf("\n", s - 1) + 1));

    if (e.key === "ArrowUp" && b.start > 0 && visualRow(s).first) {
      e.preventDefault();
      setActive(b.start - 1, col);
    } else if (e.key === "ArrowDown" && b.end < lines.length - 1 && visualRow(s).last) {
      e.preventDefault();
      setActive(b.end + 1, col);
    } else if (e.key === "ArrowLeft" && s === 0 && b.start > 0) {
      e.preventDefault();
      setActive(b.start - 1, lines[b.start - 1].length);
    } else if (e.key === "ArrowRight" && s === ta.value.length && b.end < lines.length - 1) {
      e.preventDefault();
      setActive(b.end + 1, 0);
    }
  }

  function focusout(e: FocusEvent) {
    const next = e.relatedTarget as Node | null;
    if (next && container.contains(next)) return;
    if (!document.hasFocus()) return; // window switch, keep the caret block
    activeLine = null;
  }

  function click(b: Block, e: MouseEvent) {
    if ((e.target as HTMLElement).closest("a")) return;
    if (window.getSelection() && !window.getSelection()!.isCollapsed) return;
    setActive(b.start, "end");
  }
</script>

<div bind:this={container} class="live" role="group" tabindex="-1" onfocusout={focusout}>
  {#each blocks as b (b.start)}
    {#if b.start === activeStart}
      <div class="md-block {cls(b)}">
        <div class="md-stack">
          <div class="md-backdrop" aria-hidden="true">{@html Markdown.renderLive(b.text)}</div>
          <textarea
            class="md-input"
            value={b.text}
            placeholder={isEmpty && b.start === 0 ? placeholder : ""}
            rows="1"
            spellcheck="false"
            aria-label="Entry text"
            oninput={(e) => onInput(b, e)}
            onbeforeinput={(e) => onBeforeInput(b, e)}
            onkeydown={(e) => onKey(b, e)}
          ></textarea>
        </div>
      </div>
    {:else}
      <div
        class="md-block {cls(b)}"
        role="button"
        tabindex={b.start === 0 ? 0 : -1}
        onclick={(e) => click(b, e)}
        onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); setActive(b.start, "end"); } }}
      >
        {#if isEmpty && b.start === 0}
          <span class="md-placeholder">{placeholder}</span>
        {:else}
          {@html renderBlock(b.text, b.fence)}
        {/if}
      </div>
    {/if}
  {/each}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="live-tail" role="presentation" onclick={focusEnd}></div>
</div>
