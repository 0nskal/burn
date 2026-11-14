// A shared "now" that ticks, so reset times and "synced 3 min ago" stay current while open.

export const clock = $state({ now: Date.now() });

setInterval(() => (clock.now = Date.now()), 15_000);
