// Pure settings changes (the setters of `Settings.swift`). Each returns a new object; the caller
// shows it right away and sends the whole thing to the app.

import { customSizeOf, defaultWindow, merge, order, sizes } from "./layout";
import type { CardColor, CardSize, PanelLayout, PanelOrder, ProviderId, Settings } from "./types";

const without = (list: ProviderId[], p: ProviderId) => list.filter((x) => x !== p);

export const showsInPanel = (s: Settings, p: ProviderId) => !s.panelHidden.includes(p);

export function setShowsInPanel(s: Settings, p: ProviderId, on: boolean): Settings {
  return { ...s, panelHidden: on ? without(s.panelHidden, p) : [...without(s.panelHidden, p), p] };
}

export const showsInWidget = (s: Settings, p: ProviderId) => !s.widgetHidden.includes(p);

export function setShowsInWidget(s: Settings, p: ProviderId, on: boolean): Settings {
  return { ...s, widgetHidden: on ? without(s.widgetHidden, p) : [...without(s.widgetHidden, p), p] };
}

/** Window id shown in the taskbar, or `highest`. */
export const widgetWindow = (s: Settings, p: ProviderId) => s.widgetWindows[p] ?? defaultWindow(p);

export function setWidgetWindow(s: Settings, p: ProviderId, id: string): Settings {
  return { ...s, widgetWindows: { ...s.widgetWindows, [p]: id } };
}

/** Window id shown big on the card, or `highest`. Separate from the taskbar choice. */
export const cardWindowId = (s: Settings, p: ProviderId) => s.cardWindows[p] ?? defaultWindow(p);

export function setCardWindow(s: Settings, p: ProviderId, id: string): Settings {
  return { ...s, cardWindows: { ...s.cardWindows, [p]: id } };
}

export function setCardColor(s: Settings, p: ProviderId, color: CardColor): Settings {
  return { ...s, cardColors: { ...s.cardColors, [p]: color } };
}

/** Outside Custom, first saves what `cards` look like now and switches to Custom, so only `p` changes. */
export function setCardSize(s: Settings, p: ProviderId, size: CardSize, cards: ProviderId[]): Settings {
  const cardSizes = { ...s.cardSizes };
  let panelLayout = s.panelLayout;
  if (panelLayout !== "custom") {
    for (const [q, sz] of sizes(panelLayout, cards, (x) => customSizeOf(s, x))) cardSizes[q] = sz;
    panelLayout = "custom";
  }
  cardSizes[p] = size;
  return { ...s, cardSizes, panelLayout };
}

/** Takes `shown` as the new order of those providers and switches to manual order, so a drag freezes what was on screen. */
export function reorder(s: Settings, shown: ProviderId[]): Settings {
  return { ...s, providerOrder: merge(shown, order(s.providerOrder)), panelOrder: "manual" };
}

export const setShowsSpending = (s: Settings, on: boolean): Settings => ({ ...s, showsSpending: on });
export const setPanelLayout = (s: Settings, panelLayout: PanelLayout): Settings => ({ ...s, panelLayout });
export const setPanelOrder = (s: Settings, panelOrder: PanelOrder): Settings => ({ ...s, panelOrder });
export const setTrayOnly = (s: Settings, trayOnly: boolean): Settings => ({ ...s, trayOnly });
