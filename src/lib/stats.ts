import { addDays, dayKey } from "./dates";
import type { Entry } from "./journal.svelte";

export const words = (s: string) => (s.trim() ? s.trim().split(/\s+/).length : 0);

export function computeStats(all: Entry[]) {
  const list = all.filter((e) => !e.draft);
  const byDay = new Map<number, { entries: number; words: number }>();
  const hours = Array<number>(24).fill(0);
  const weekdays = Array<number>(7).fill(0); // Monday first
  let totalWords = 0;

  for (const e of list) {
    const w = words(e.body);
    totalWords += w;
    const k = dayKey(e.created_at);
    const d = byDay.get(k) ?? { entries: 0, words: 0 };
    d.entries++;
    d.words += w;
    byDay.set(k, d);
    const dt = new Date(e.created_at * 1000);
    hours[dt.getHours()]++;
    weekdays[(dt.getDay() + 6) % 7]++;
  }

  const today = dayKey(Date.now() / 1000);

  // current streak: today counts if written, otherwise it can still be saved
  let current = 0;
  for (let k = byDay.has(today) ? today : addDays(today, -1); byDay.has(k); k = addDays(k, -1)) current++;

  let longest = 0, run = 0, prev = 0;
  for (const k of [...byDay.keys()].sort((a, b) => a - b)) {
    run = prev && addDays(prev, 1) === k ? run + 1 : 1;
    longest = Math.max(longest, run);
    prev = k;
  }

  // last 20 weeks, Monday-first columns
  const dow = (new Date(today).getDay() + 6) % 7;
  const start = addDays(today, -dow - 19 * 7);
  const weeks = Array.from({ length: 20 }, (_, w) =>
    Array.from({ length: 7 }, (_, d) => {
      const key = addDays(start, w * 7 + d);
      const v = byDay.get(key);
      return { key, words: v?.words ?? 0, entries: v?.entries ?? 0, future: key > today };
    })
  );

  // words per month, last 6 months
  const now = new Date();
  const months = Array.from({ length: 6 }, (_, i) => {
    const d = new Date(now.getFullYear(), now.getMonth() - (5 - i), 1);
    let w = 0;
    for (const [k, v] of byDay) {
      const kd = new Date(k);
      if (kd.getFullYear() === d.getFullYear() && kd.getMonth() === d.getMonth()) w += v.words;
    }
    return { label: d.toLocaleDateString("en-GB", { month: "short" }), words: w };
  });

  const monthStart = new Date(now.getFullYear(), now.getMonth(), 1).getTime() / 1000;
  const max = (a: number[]) => a.indexOf(Math.max(...a));

  return {
    entries: list.length,
    words: totalWords,
    avgWords: list.length ? Math.round(totalWords / list.length) : 0,
    activeDays: byDay.size,
    thisMonth: list.filter((e) => e.created_at >= monthStart).length,
    currentStreak: current,
    longestStreak: longest,
    weekdays,
    hours,
    weeks,
    months,
    busiestWeekday: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"][max(weekdays)],
    peakHour: max(hours),
  };
}
