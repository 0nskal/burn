<!-- The panel (PopoverView.swift): provider cards, spend, and a footer, floating over the desktop. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { contentResized, getState, hidePanel, onPanel, onState, setPeriod } from "./lib/api";
  import { clock } from "./lib/clock.svelte";
  import { demoUsage } from "./lib/demo";
  import { synced } from "./lib/format";
  import { customSize, hasCard, order, panelProviders, rows as arrange, sizes } from "./lib/layout";
  import { pageIn, pageOut } from "./lib/motion";
  import { holdToEdit } from "./lib/press";
  import { reorder, setCardSize, setShowsInPanel, setShowsSpending } from "./lib/settings";
  import { LOGIN_COMMAND, PROVIDERS, PROVIDER_NAME, type CardSize, type Period, type ProviderId, type Settings, type Snapshot } from "./lib/types";
  import CardGrid from "./lib/components/CardGrid.svelte";
  import CommandChip from "./lib/components/CommandChip.svelte";
  import EditChrome from "./lib/components/EditChrome.svelte";
  import Glyph from "./lib/components/Glyph.svelte";
  import Icons from "./lib/components/Icons.svelte";
  import IssueRow from "./lib/components/IssueRow.svelte";
  import RoundButton from "./lib/components/RoundButton.svelte";
  import SettingsView from "./lib/components/SettingsView.svelte";
  import SpendBlock from "./lib/components/SpendBlock.svelte";
  import Tip from "./lib/components/Tip.svelte";
  import { isTauri, quit, setLaunchAtLogin, setSettings } from "./lib/api";

  let snap = $state<Snapshot | null>(null);
  /** False while the panel is closed, so the content animates in on every open. */
  let visible = $state(false);
  let root: HTMLDivElement | undefined = $state();
  let view = $state<"usage" | "settings">("usage");
  /** The provider page open in settings. */
  let settingsProvider = $state<ProviderId | null>(null);
  let editing = $state(false);

  const shown = $derived(snap ? panelProviders(snap) : []);
  const cards = $derived(snap ? shown.filter((p) => hasCard(snap!, p)) : []);
  const issues = $derived(shown.filter((p) => !cards.includes(p)));
  const sizeOf = $derived(snap ? sizes(snap.settings.panelLayout, cards, (p) => customSize(snap!, p)) : new Map());
  const cardRows = $derived(arrange(cards, sizeOf));
  const spendShown = $derived(snap != null && snap.settings.showsSpending && (snap.hasUsage || snap.importPercent != null));

  /** Hidden providers that are signed in, and whether the Spent block is hidden: what "+" brings back. */
  const hiddenProviders = $derived(snap ? order(snap.settings.providerOrder).filter((p) => snap!.connected.includes(p) && snap!.settings.panelHidden.includes(p)) : []);
  const panelEmpty = $derived(snap != null && snap.connected.length > 0 && shown.length === 0 && !spendShown);

  /** Shows the change right away, then hands the whole settings object to the app. */
  function commit(next: Settings) {
    if (!snap) return;
    snap.settings = next;
    setSettings(next);
  }

  function launchAtLogin(on: boolean) {
    if (!snap) return;
    snap.launchAtLogin = on;
    setLaunchAtLogin(on);
  }

  const startEditing = () => {
    view = "usage";
    editing = true;
  };
  const stopEditing = () => (editing = false);

  function choosePeriod(p: Period) {
    if (!snap) return;
    snap.period = p;
    if (isTauri) setPeriod(p);
    else snap.usage = demoUsage(p);
  }

  onMount(() => {
    let cancelled = false;
    (async () => {
      // Labels are measured (segmented control, panel size): wait for the brand font first.
      await document.fonts.ready;
      const first = await getState();
      if (cancelled) return;
      snap = first;
      await onState((s) => (snap = s));
      await onPanel(
        () => {
          clock.now = Date.now();
          visible = false;
          requestAnimationFrame(() => requestAnimationFrame(() => (visible = true)));
        },
        () => {
          visible = false;
          // The panel opens on the usage page again, never in the middle of editing.
          editing = false;
          view = "usage";
          settingsProvider = null;
        },
      );
      // Development aid for screenshots: `#settings`, `#settings/claude` or `#edit` opens there.
      const [start, provider] = location.hash.slice(1).split("/");
      if (start === "settings") {
        view = "settings";
        settingsProvider = (provider as ProviderId) || null;
      } else if (start === "edit") editing = true;
    })();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (editing) editing = false;
        else hidePanel();
      } else if (e.key === "Enter" && editing && !(e.target as Element).closest("button, select, [data-card]")) {
        editing = false;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      cancelled = true;
      window.removeEventListener("keydown", onKey);
    };
  });

  // The window is sized to the content, so it never covers more of the desktop than it shows.
  $effect(() => {
    if (!root) return;
    const el = root;
    const observer = new ResizeObserver(() => contentResized(el.offsetWidth, el.offsetHeight));
    observer.observe(el);
    return () => observer.disconnect();
  });
</script>

<div class="root" bind:this={root}>
  {#if snap}
    <div class="content" class:visible>
      <div class="stage">
        {#if view === "settings"}
          <div class="page" in:pageIn out:pageOut>
            <div class="block settings">
              <SettingsView
                {snap}
                bind:provider={settingsProvider}
                {commit}
                onLaunch={launchAtLogin}
                onBack={() => (view = "usage")}
                onArrange={startEditing}
                onQuit={quit}
              />
            </div>
          </div>
        {:else}
          <div class="page" in:pageIn out:pageOut>
            {#if snap.connected.length === 0}
              <div class="block neutral reveal" style:--i={0}>
                <p class="hint">Sign in to a provider in a terminal:</p>
                {#each PROVIDERS as p (p)}
                  <div class="signin-row"><Glyph provider={p} size={12} /><CommandChip command={LOGIN_COMMAND[p]} /></div>
                {/each}
              </div>
            {:else}
              <CardGrid
                {snap}
                {cards}
                rows={cardRows}
                {sizeOf}
                {editing}
                onEdit={startEditing}
                onHide={(p) => commit(setShowsInPanel(snap!.settings, p, false))}
                onSize={(p, size: CardSize) => commit(setCardSize(snap!.settings, p, size, cards))}
                onReorder={(next) => commit(reorder(snap!.settings, next))}
              />
              {#each issues as p (p)}
                <div class="reveal" style:--i={cardRows.length} use:holdToEdit={editing ? null : startEditing}>
                  <EditChrome {editing} title={PROVIDER_NAME[p]} seed={PROVIDERS.indexOf(p) * 1.9} onHide={() => commit(setShowsInPanel(snap!.settings, p, false))}>
                    <IssueRow provider={p} state={snap.limits[p] ?? { state: "loading" }} />
                  </EditChrome>
                </div>
              {/each}
            {/if}

            {#if spendShown}
              <div class="reveal" style:--i={cardRows.length + 1} use:holdToEdit={editing ? null : startEditing}>
                <EditChrome {editing} title="Spent" seed={5.3} onHide={() => commit(setShowsSpending(snap!.settings, false))}>
                  {#if snap.hasUsage}
                    <SpendBlock usage={snap.usage} error={snap.usageError} period={snap.period} onPeriod={choosePeriod} />
                  {:else}
                    <div class="block importing">
                      <span>Importing usage from Claude Code and Codex logs…</span>
                      <div class="progress"><div style:width="{snap.importPercent ?? 0}%"></div></div>
                    </div>
                  {/if}
                </EditChrome>
              </div>
            {/if}

            {#if editing && (hiddenProviders.length > 0 || !snap.settings.showsSpending)}
              <div class="hidden-list">
                {#each hiddenProviders as p (p)}
                  <button class="add" aria-label="Show {PROVIDER_NAME[p]}" onclick={() => commit(setShowsInPanel(snap!.settings, p, true))}>
                    <Icons name="plus" size={9} />{PROVIDER_NAME[p]}
                  </button>
                {/each}
                {#if !snap.settings.showsSpending}
                  <button class="add" aria-label="Show Spent" onclick={() => commit(setShowsSpending(snap!.settings, true))}>
                    <Icons name="plus" size={9} />Spent
                  </button>
                {/if}
              </div>
            {:else if !editing && panelEmpty}
              <div class="block neutral empty reveal" style:--i={0}>
                <p class="hint">Everything is hidden from the panel</p>
                <button class="pill-button prominent" onclick={startEditing}>Edit panel</button>
              </div>
            {/if}

            <footer class="footer reveal" style:--i={cardRows.length + 2}>
              <div class="row-in" class:off={editing} inert={editing}>
                <span class="status" class:hidden={!snap.hasUsage}>{snap.syncing ? "Syncing…" : synced(snap.lastSync, clock.now)}</span>
                <RoundButton icon="gear" label="Settings" onclick={() => (view = "settings")} />
              </div>
              <div class="row-in" class:off={!editing} inert={!editing}>
                <span class="status">Drag to reorder</span>
                <button class="pill-button prominent" onclick={stopEditing}>Done</button>
              </div>
            </footer>
          </div>
        {/if}
      </div>
    </div>
  {/if}
  <Tip />
</div>

<style>
  /* Room around the blocks for their shadows (Bento.shadowInset), and on top for the edit chrome's hide buttons. */
  .root {
    width: 332px;
    padding: 12px 14px 18px;
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    transform-origin: bottom center;
    transform: scale(0.96);
    opacity: 0;
  }
  .content.visible {
    transform: none;
    opacity: 1;
    transition:
      transform 200ms var(--ease-out),
      opacity 200ms var(--ease-out);
  }
  /* Blocks fade and lift in, staggered (Bento.swift `Reveal`). Only on the way in. */
  .stage {
    position: relative;
  }
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
  }
  .settings {
    background: var(--surface);
  }
  :global(.reveal) {
    opacity: 0;
    transform: translateY(6px);
  }
  .visible :global(.reveal) {
    opacity: 1;
    transform: none;
    transition:
      opacity 220ms var(--ease-out),
      transform 220ms var(--ease-out);
    transition-delay: calc(var(--i, 0) * 30ms);
  }
  .empty {
    align-items: center;
    padding: 18px 12px;
  }
  .hidden-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--gap);
    animation: fade-in 180ms var(--ease-out);
  }
  .add {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 32px;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--surface);
    font-size: 12px;
    font-weight: 700;
    box-shadow:
      0 4px 20px var(--shadow),
      inset 0 0 0 0.5px var(--hairline);
    transition: transform 120ms ease-out;
  }
  .add::after {
    content: "";
    position: absolute;
    inset: -4px -2px;
  }
  .add:active {
    transform: scale(0.97);
  }
  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }
  .neutral {
    background: var(--neutral);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .hint {
    color: var(--secondary);
  }
  .signin-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .importing {
    background: var(--lime);
    color: var(--lime-ink);
    font-size: 11px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .progress {
    height: 4px;
    border-radius: 2px;
    background: color-mix(in srgb, var(--lime-ink) 15%, transparent);
    overflow: hidden;
  }
  .progress div {
    height: 100%;
    background: var(--lime-ink);
    transition: width 300ms var(--ease-out);
  }
  .footer {
    display: grid;
    min-height: 36px;
    margin-top: 2px;
    padding: 4px 4px 4px 12px;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      0 4px 20px var(--shadow),
      inset 0 0 0 0.5px var(--hairline);
    font-size: 11px;
  }
  /* Both rows sit in the same cell and crossfade, so the footer never changes size. */
  .row-in {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--gap);
    transition: opacity 150ms ease-out;
  }
  .row-in.off {
    opacity: 0;
    pointer-events: none;
  }
  .status {
    color: var(--secondary);
  }
  .status.hidden {
    opacity: 0;
  }
</style>
