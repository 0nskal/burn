// Mirrors the Rust types the app sends (`burn-app/src/model.rs`, `burn-core/src/model.rs`).

export type ProviderId = "claude" | "codex" | "cursor";
export const PROVIDERS: ProviderId[] = ["claude", "codex", "cursor"];
export const PROVIDER_NAME: Record<ProviderId, string> = { claude: "Claude", codex: "Codex", cursor: "Cursor" };
export const LOGIN_COMMAND: Record<ProviderId, string> = {
  claude: "claude auth login",
  codex: "codex login",
  cursor: "cursor-agent login",
};

export interface LimitWindow {
  id: string;
  label: string;
  /** Used, 0...100 (may exceed 100). */
  percent: number;
  /** Epoch ms. */
  resetsAt: number | null;
  minor: boolean;
}

export interface ProviderLimits {
  provider: ProviderId;
  plan: string | null;
  windows: LimitWindow[];
  fetchedAt: number;
}

export type LimitState =
  | { state: "loading" }
  | { state: "loaded"; limits: ProviderLimits }
  | { state: "signedOut" }
  | { state: "failed"; message: string; last: ProviderLimits | null };

export type Period = "today" | "week" | "month" | "all";
export const PERIODS: Period[] = ["today", "week", "month", "all"];
export const PERIOD_TITLE: Record<Period, string> = { today: "Today", week: "7D", month: "30D", all: "All" };

export type BucketUnit = "hour" | "day" | "week";

export interface UsageBucket {
  start: number;
  cost: Partial<Record<ProviderId, number>>;
}

export interface UsageSummary {
  period: Period;
  cost: number;
  tokens: number;
  byProvider: Partial<Record<ProviderId, number>>;
  buckets: UsageBucket[];
  unit: BucketUnit;
}

export type PanelLayout = "auto" | "grid" | "list" | "custom";
export type PanelOrder = "manual" | "closestToLimit";
export type CardSize = "s" | "m" | "l";
export type CardColor = "coral" | "mono" | "lavender" | "sky";
export type Tab = "usage" | "shelf";

export interface Settings {
  widgetHidden: ProviderId[];
  widgetWindows: Partial<Record<ProviderId, string>>;
  panelHidden: ProviderId[];
  cardSizes: Partial<Record<ProviderId, CardSize>>;
  cardColors: Partial<Record<ProviderId, CardColor>>;
  cardWindows: Partial<Record<ProviderId, string>>;
  providerOrder: ProviderId[];
  panelLayout: PanelLayout;
  panelOrder: PanelOrder;
  showsSpending: boolean;
  tab: Tab;
  shelfExpanded: boolean;
  shelfRetentionDays: number;
  trayOnly: boolean;
}

export interface Snapshot {
  connected: ProviderId[];
  limits: Partial<Record<ProviderId, LimitState>>;
  usage: UsageSummary | null;
  usageError: string | null;
  period: Period;
  lastSync: number | null;
  syncing: boolean;
  importPercent: number | null;
  hasUsage: boolean;
  settings: Settings;
}

export function limitsOf(s: LimitState | undefined): ProviderLimits | null {
  if (!s) return null;
  if (s.state === "loaded") return s.limits;
  if (s.state === "failed") return s.last;
  return null;
}
