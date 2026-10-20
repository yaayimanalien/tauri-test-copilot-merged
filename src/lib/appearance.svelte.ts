// Theme, accent and writing font. Each maps to a data-* attribute on <html>
// (see the token blocks in app.css), so adding an option is one CSS block.
export type Theme = "dark" | "light" | "system";
export type Accent = "blue" | "purple" | "pink" | "red" | "orange" | "green";
export type Font = "system" | "serif" | "mono";
export type GlassStyle = "liquid" | "frosted" | "tinted";

export const ACCENTS: Accent[] = ["blue", "purple", "pink", "red", "orange", "green"];

const KEY = "daylight.appearance";

function load() {
  const base = {
    theme: "dark" as Theme,
    accent: "blue" as Accent,
    font: "system" as Font,
    glass: "liquid" as GlassStyle,
  };
  try {
    return { ...base, ...JSON.parse(localStorage.getItem(KEY) ?? "{}") };
  } catch {
    return base;
  }
}

export const appearance = $state(load());

function apply() {
  const root = document.documentElement;
  const resolved =
    appearance.theme === "system"
      ? matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark"
      : appearance.theme;
  root.dataset.theme = resolved;
  root.dataset.accent = appearance.accent;
  root.dataset.font = appearance.font;
  root.dataset.glass = appearance.glass;
  root.classList.toggle("dark", resolved === "dark");
}

export function setAppearance(patch: Partial<typeof appearance>) {
  Object.assign(appearance, patch);
  try { localStorage.setItem(KEY, JSON.stringify({ ...appearance })); } catch { /* ignore */ }
  apply();
}

export function initAppearance() {
  apply();
  matchMedia("(prefers-color-scheme: light)").addEventListener("change", () => {
    if (appearance.theme === "system") apply();
  });
}
