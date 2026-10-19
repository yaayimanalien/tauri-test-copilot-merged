/* Tiny dependency-free Markdown renderer.
 * Markdown.render(text)     -> HTML for the read view
 * Markdown.renderLive(text) -> HTML for the editor backdrop. Every source
 *   character is kept (markers are only dimmed) so it lines up exactly with
 *   the transparent textarea on top of it.
 * All input is HTML-escaped first, so no raw HTML can get through. */
const Markdown = (function () {
  "use strict";

  function esc(s) {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  // Only allow safe schemes. `u` is already HTML-escaped.
  function safeUrl(u) {
    return /^(https?:\/\/|mailto:)/i.test(u);
  }

  /* ---------- inline (read view) ---------- */
  function inline(src) {
    const codes = [];
    let s = src.replace(/`([^`\n]+)`/g, (_, c) => {
      codes.push(c);
      return "\u0000" + (codes.length - 1) + "\u0000";
    });
    s = esc(s);
    s = s.replace(/!\[([^\]]*)\]\(([^)\s]+)\)/g, (m, alt, url) =>
      safeUrl(url) ? '<img src="' + url + '" alt="' + alt + '" loading="lazy">' : m
    );
    s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (m, text, url) =>
      safeUrl(url)
        ? '<a href="' + url + '" target="_blank" rel="noopener noreferrer">' + text + "</a>"
        : m
    );
    s = s.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>");
    s = s.replace(/__([^_\n]+)__/g, "<strong>$1</strong>");
    s = s.replace(/(^|[^*\w])\*([^*\s][^*\n]*?)\*(?!\*)/g, "$1<em>$2</em>");
    s = s.replace(/(^|[^_\w])_([^_\s][^_\n]*?)_(?![_\w])/g, "$1<em>$2</em>");
    s = s.replace(/~~([^~\n]+)~~/g, "<del>$1</del>");
    return s.replace(/\u0000(\d+)\u0000/g, (_, i) => "<code>" + esc(codes[+i]) + "</code>");
  }

  /* ---------- blocks (read view) ---------- */
  function render(text) {
    const lines = String(text || "").replace(/\r\n?/g, "\n").split("\n");
    const out = [];
    let i = 0;

    while (i < lines.length) {
      const line = lines[i];

      if (/^\s*$/.test(line)) { i++; continue; }

      const fence = line.match(/^```\s*([\w+-]*)\s*$/);
      if (fence) {
        const buf = [];
        i++;
        while (i < lines.length && !/^```\s*$/.test(lines[i])) buf.push(lines[i++]);
        i++; // closing fence (or EOF)
        out.push("<pre><code>" + esc(buf.join("\n")) + "</code></pre>");
        continue;
      }

      const h = line.match(/^(#{1,6})\s+(.*)$/);
      if (h) {
        out.push("<h" + h[1].length + ">" + inline(h[2].trim()) + "</h" + h[1].length + ">");
        i++;
        continue;
      }

      if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) { out.push("<hr>"); i++; continue; }

      if (/^>\s?/.test(line)) {
        const buf = [];
        while (i < lines.length && /^>\s?/.test(lines[i])) buf.push(lines[i++].replace(/^>\s?/, ""));
        out.push("<blockquote>" + render(buf.join("\n")) + "</blockquote>");
        continue;
      }

      const li = line.match(/^\s*([-*+]|\d+\.)\s+(.*)$/);
      if (li) {
        const ordered = /\d/.test(li[1]);
        const items = [];
        while (i < lines.length) {
          const m = lines[i].match(/^\s*([-*+]|\d+\.)\s+(.*)$/);
          if (!m || /\d/.test(m[1]) !== ordered) break;
          items.push("<li>" + inline(m[2]) + "</li>");
          i++;
        }
        const tag = ordered ? "ol" : "ul";
        out.push("<" + tag + ">" + items.join("") + "</" + tag + ">");
        continue;
      }

      const buf = [];
      while (
        i < lines.length &&
        !/^\s*$/.test(lines[i]) &&
        !/^(#{1,6}\s|```|>\s?|\s*([-*+]|\d+\.)\s)/.test(lines[i])
      ) buf.push(inline(lines[i++]));
      out.push("<p>" + buf.join("<br>") + "</p>");
    }
    return out.join("");
  }

  /* ---------- live editor backdrop ---------- */
  const M = (t) => '<span class="m">' + t + "</span>";

  function liveInline(s) {
    s = esc(s);
    s = s.replace(/`([^`\n]+)`/g, (_, c) => M("`") + '<span class="lc">' + c + "</span>" + M("`"));
    s = s.replace(/\*\*([^*\n]+)\*\*/g, (_, c) => M("**") + '<span class="lb">' + c + "</span>" + M("**"));
    s = s.replace(/(^|[^*\w])\*([^*\s][^*\n]*?)\*(?!\*)/g, (_, p, c) => p + M("*") + '<span class="li">' + c + "</span>" + M("*"));
    s = s.replace(/~~([^~\n]+)~~/g, (_, c) => M("~~") + '<span class="ls">' + c + "</span>" + M("~~"));
    s = s.replace(/\[([^\]\n]+)\]\(([^)\s]+)\)/g, (_, t, u) => M("[") + '<span class="lk">' + t + "</span>" + M("](" + u + ")"));
    return s;
  }

  function renderLive(text) {
    const lines = String(text || "").replace(/\r\n?/g, "\n").split("\n");
    let inFence = false;
    const out = lines.map((line) => {
      if (/^```/.test(line)) {
        inFence = !inFence;
        return '<span class="m">' + esc(line) + "</span>";
      }
      if (inFence) return '<span class="lc">' + esc(line) + "</span>";
      let m;
      if ((m = line.match(/^(#{1,6}\s)(.*)$/))) return M(esc(m[1])) + '<span class="lh lh' + m[1].trim().length + '">' + liveInline(m[2]) + "</span>";
      if ((m = line.match(/^(>\s?)(.*)$/))) return M(esc(m[1])) + '<span class="lq">' + liveInline(m[2]) + "</span>";
      if ((m = line.match(/^(\s*(?:[-*+]|\d+\.)\s)(.*)$/))) return M(esc(m[1])) + liveInline(m[2]);
      if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) return M(esc(line));
      return liveInline(line);
    });
    // Trailing zero-width space keeps the last empty line's height.
    return out.join("\n") + "​";
  }

  return { render, renderLive };
})();

export { Markdown };
