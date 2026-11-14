import { describe, expect, it } from "vitest";
import { demoSnapshot } from "./demo";
import {
  cardWindowId,
  reorder,
  setCardColor,
  setCardSize,
  setCardWindow,
  setShowsInPanel,
  setShowsInWidget,
  setShelfExpanded,
  setShelfRetentionDays,
  setShowsSpending,
  setTab,
  setWidgetWindow,
  showsInPanel,
  showsInWidget,
  widgetWindow,
} from "./settings";
import type { Settings } from "./types";

const base = (): Settings => ({ ...demoSnapshot(null).settings, providerOrder: ["claude", "codex", "cursor"] });

describe("settings changes", () => {
  it("never mutates its input", () => {
    const s = base();
    const copy = structuredClone(s);
    setShowsInPanel(s, "codex", false);
    reorder(s, ["cursor", "codex", "claude"]);
    setCardSize(s, "codex", "s", ["claude", "codex"]);
    expect(s).toEqual(copy);
  });

  it("hides and shows in the panel and the widget", () => {
    let s = setShowsInPanel(base(), "codex", false);
    expect(showsInPanel(s, "codex")).toBe(false);
    s = setShowsInPanel(setShowsInPanel(s, "codex", false), "cursor", false);
    expect(s.panelHidden).toEqual(["codex", "cursor"]);
    expect(showsInPanel(setShowsInPanel(s, "codex", true), "codex")).toBe(true);

    const w = setShowsInWidget(base(), "claude", false);
    expect(showsInWidget(w, "claude")).toBe(false);
    expect(showsInWidget(setShowsInWidget(w, "claude", true), "claude")).toBe(true);
  });

  it("windows default per provider and can be chosen", () => {
    const s = base();
    expect(widgetWindow(s, "claude")).toBe("5h");
    expect(widgetWindow(s, "cursor")).toBe("total");
    expect(widgetWindow(s, "codex")).toBe("highest");
    expect(widgetWindow(setWidgetWindow(s, "claude", "week"), "claude")).toBe("week");
    // The card and the taskbar choose separately.
    const c = setCardWindow(s, "claude", "highest");
    expect(cardWindowId(c, "claude")).toBe("highest");
    expect(widgetWindow(c, "claude")).toBe("5h");
  });

  it("sets a card color", () => {
    expect(setCardColor(base(), "codex", "sky").cardColors).toEqual({ codex: "sky" });
  });

  it("changing a size in Custom keeps the other sizes", () => {
    const s = setCardSize({ ...base(), panelLayout: "custom" }, "codex", "s", ["claude", "codex", "cursor"]);
    expect(s.cardSizes).toEqual({ codex: "s" });
    expect(s.panelLayout).toBe("custom");
  });

  it("changing a size outside Custom saves the current sizes first", () => {
    const s = setCardSize({ ...base(), panelLayout: "auto" }, "codex", "m", ["claude", "codex", "cursor"]);
    expect(s.panelLayout).toBe("custom");
    // Auto with three cards was L, S, S; only codex changed.
    expect(s.cardSizes).toEqual({ claude: "l", codex: "m", cursor: "s" });
  });

  it("reorder merges into the saved order and switches to manual", () => {
    const s = reorder({ ...base(), panelOrder: "closestToLimit" }, ["cursor", "claude"]);
    expect(s.panelOrder).toBe("manual");
    // Codex was hidden from the shown list, so it keeps its slot.
    expect(s.providerOrder).toEqual(["cursor", "codex", "claude"]);
  });

  it("toggles spending", () => {
    expect(setShowsSpending(base(), false).showsSpending).toBe(false);
  });

  it("changes the tab, shelf size and retention without touching the rest", () => {
    const s = base();
    const copy = structuredClone(s);
    expect(setTab(s, "shelf")).toEqual({ ...s, tab: "shelf" });
    expect(setShelfExpanded(s, true)).toEqual({ ...s, shelfExpanded: true });
    expect(setShelfRetentionDays(s, 0)).toEqual({ ...s, shelfRetentionDays: 0 });
    expect(s).toEqual(copy);
  });
});
