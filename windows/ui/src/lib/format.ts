// Port of `Format.swift`. English, like the Mac app.

const money2 = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", minimumFractionDigits: 2, maximumFractionDigits: 2 });
const money0 = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", minimumFractionDigits: 0, maximumFractionDigits: 0 });

export function money(v: number): string {
  return (v >= 100 ? money0 : money2).format(v);
}

export function tokens(v: number): string {
  // Thresholds after rounding, so 999,600 reads "1M", not "1000K".
  if (v >= 999_950_000) return `${(v / 1e9).toFixed(1)}B`;
  if (v >= 999_500) return `${(v / 1e6).toFixed(0)}M`;
  if (v >= 1e3) return `${(v / 1e3).toFixed(0)}K`;
  return `${v}`;
}

/** Share of a limit still left, 0...100, from its used percent. */
export function remaining(used: number): number {
  return Math.round(Math.min(Math.max(100 - used, 0), 100));
}

const weekday = new Intl.DateTimeFormat("en-US", { weekday: "short" });
const dayMonth = (d: Date) => `${d.getDate()} ${d.toLocaleString("en-US", { month: "short" })}`;

/** "2h 10m" within a day, weekday within a week, otherwise "1 Nov". */
export function reset(at: number | null, now = Date.now()): string {
  if (at == null) return "";
  const s = (at - now) / 1000;
  if (s <= 0) return "now";
  if (s < 3600) return `${Math.max(1, Math.floor(s / 60))}m`;
  if (s < 86400) {
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    return m === 0 ? `${h}h` : `${h}h ${m}m`;
  }
  const d = new Date(at);
  return s < 6 * 86400 ? weekday.format(d) : dayMonth(d);
}

/** True when the reset is within the next day, so "resets in 2h" reads right. */
export function resetsSoon(at: number | null, now = Date.now()): boolean {
  if (at == null) return false;
  const s = (at - now) / 1000;
  return s >= 1 && s < 86400;
}

export function synced(at: number | null, now = Date.now()): string {
  if (at == null) return "Not synced yet";
  const s = (now - at) / 1000;
  if (s < 60) return "Synced just now";
  if (s < 3600) return `Synced ${Math.floor(s / 60)} min ago`;
  return `Synced ${Math.floor(s / 3600)} h ago`;
}

const pad = (n: number) => String(n).padStart(2, "0");

export function bucketLabel(start: number | undefined, unit: "hour" | "day" | "week"): string {
  if (start == null) return "";
  const d = new Date(start);
  switch (unit) {
    case "hour":
      return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
    case "day":
      return dayMonth(d);
    case "week":
      return `${d.toLocaleString("en-US", { month: "short" })} ${d.getFullYear()}`;
  }
}

/** "Tue, 3 Mar" / "14:00" / "Week of 3 Mar", for chart tips. */
export function bucketTitle(start: number, unit: "hour" | "day" | "week"): string {
  const d = new Date(start);
  switch (unit) {
    case "hour":
      return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
    case "day":
      return `${weekday.format(d)}, ${dayMonth(d)}`;
    case "week":
      return `Week of ${dayMonth(d)}`;
  }
}
