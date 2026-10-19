const p = (n: number) => String(n).padStart(2, "0");

/** Start of the local day, in ms. Used as the key for grouping. */
export function dayKey(ts: number): number {
  const d = new Date(ts * 1000);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

export function addDays(key: number, n: number): number {
  const d = new Date(key);
  d.setDate(d.getDate() + n);
  return d.getTime();
}

export function timeLabel(ts: number): string {
  const d = new Date(ts * 1000);
  return p(d.getHours()) + ":" + p(d.getMinutes());
}

export function dayParts(key: number) {
  const d = new Date(key);
  return {
    num: d.getDate(),
    weekday: d.toLocaleDateString("en-GB", { weekday: "long" }),
    month: d.toLocaleDateString("en-GB", { month: "long", year: "numeric" }),
  };
}

/** "Today", "Yesterday", "3 days ago", "2 weeks ago" … */
export function relativeDay(key: number): string {
  const today = dayKey(Date.now() / 1000);
  const diff = Math.round((today - key) / 86400000);
  if (diff <= 0) return "Today";
  if (diff === 1) return "Yesterday";
  if (diff < 7) return `${diff} days ago`;
  if (diff < 30) {
    const w = Math.round(diff / 7);
    return w === 1 ? "Last week" : `${w} weeks ago`;
  }
  if (diff < 365) {
    const m = Math.round(diff / 30);
    return m === 1 ? "Last month" : `${m} months ago`;
  }
  const y = Math.round(diff / 365);
  return y === 1 ? "Last year" : `${y} years ago`;
}
