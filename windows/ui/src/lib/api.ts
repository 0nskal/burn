// Talks to the Rust side. In a plain browser (`npm run dev` without Tauri) it serves demo data
// instead, so the UI can be built and screenshotted anywhere: `?demo=loading|signedOut|empty`.

import type { Period, Settings, Snapshot } from "./types";
import { demoSnapshot } from "./demo";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Listener = (s: Snapshot) => void;

export async function getState(): Promise<Snapshot> {
  if (!inTauri) return demoSnapshot(new URLSearchParams(location.search).get("demo"));
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Snapshot>("get_state");
}

export async function onState(f: Listener): Promise<void> {
  if (!inTauri) return;
  const { listen } = await import("@tauri-apps/api/event");
  await listen<Snapshot>("state", (e) => f(e.payload));
}

/** `shown` runs every time the panel opens, `hidden` when it closes. */
export async function onPanel(shown: () => void, hidden: () => void): Promise<void> {
  if (!inTauri) {
    shown();
    return;
  }
  const { listen } = await import("@tauri-apps/api/event");
  await listen("panel-shown", shown);
  await listen("panel-hidden", hidden);
  // The panel may have opened before the page was listening.
  const { invoke } = await import("@tauri-apps/api/core");
  if (await invoke<boolean>("panel_visible")) shown();
}

async function call(cmd: string, args?: Record<string, unknown>): Promise<void> {
  if (!inTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke(cmd, args);
}

export const setPeriod = (period: Period) => call("set_period", { period });
export const hidePanel = () => call("hide_panel");
export const contentResized = (width: number, height: number) => call("content_resized", { width, height });
export const setSettings = (settings: Settings) => call("set_settings", { settings });
export const setLaunchAtLogin = (on: boolean) => call("set_launch_at_login", { on });
export const quit = () => call("quit");
export const isTauri = inTauri;
