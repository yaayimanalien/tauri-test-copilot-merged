import { Markdown } from "./markdown";

const LIST = /^(\s*)([-*+]|(\d+)\.)\s+(.*)$/;
const HR = /^\s*([-*_])(\s*\1){2,}\s*$/;
const stripP = (html: string) => html.replace(/^<p>([\s\S]*)<\/p>$/, "$1");

/** Formats one editor block (a single line, or a whole fenced code block). */
export function renderBlock(text: string, fence: boolean): string {
  if (fence) return Markdown.render(text);
  if (!text.trim()) return "";
  if (HR.test(text)) return "<hr>";

  const m = text.match(LIST);
  if (m) {
    const indent = m[1].replace(/\t/g, "    ").length;
    const marker = m[3] ? m[3] + "." : "•";
    return (
      `<div class="md-li" style="--indent:${indent}"><span class="md-marker">${marker}</span>` +
      `<div class="md-li-body">${stripP(Markdown.render(m[4]))}</div></div>`
    );
  }
  return Markdown.render(text);
}
