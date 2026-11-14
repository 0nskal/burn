<!-- The panel (PopoverView.swift): provider cards, spend, and a footer, floating over the desktop. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { contentResized, getState, hidePanel, onPanel, onState, setPeriod } from "./lib/api";
  import { clock } from "./lib/clock.svelte";
  import { demoUsage } from "./lib/demo";
  import { synced } from "./lib/format";
  import { cardWindow, customSize, hasCard, panelProviders, rows as arrange, sizes } from "./lib/layout";
  import { BRAND } from "./lib/level";
  import { LOGIN_COMMAND, PROVIDERS, limitsOf, type Period, type Snapshot } from "./lib/types";
  import CommandChip from "./lib/components/CommandChip.svelte";
  import Glyph from "./lib/components/Glyph.svelte";
  import IssueRow from "./lib/components/IssueRow.svelte";
  import ProviderBlock from "./lib/components/ProviderBlock.svelte";
  import SpendBlock from "./lib/components/SpendBlock.svelte";
  import Tip from "./lib/components/Tip.svelte";
  import { isTauri } from "./lib/api";

  let snap = $state<Snapshot | null>(null);
  /** False while the panel is closed, so the content animates in on every open. */
  let visible = $state(false);
  let root: HTMLDivElement | undefined = $state();

  const shown = $derived(snap ? panelProviders(snap) : []);
  const cards = $derived(snap ? shown.filter((p) => hasCard(snap!, p)) : []);
  const issues = $derived(shown.filter((p) => !cards.includes(p)));
  const sizeOf = $derived(snap ? sizes(snap.settings.panelLayout, cards, (p) => customSize(snap!, p)) : new Map());
  const cardRows = $derived(arrange(cards, sizeOf));
  const spendShown = $derived(snap != null && snap.settings.showsSpending && (snap.hasUsage || snap.importPercent != null));

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
        () => (visible = false),
      );
    })();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hidePanel();
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
      {#if snap.connected.length === 0}
        <div class="block neutral reveal" style:--i={0}>
          <p class="hint">Sign in to a provider in a terminal:</p>
          {#each PROVIDERS as p (p)}
            <div class="signin-row"><Glyph provider={p} size={12} /><CommandChip command={LOGIN_COMMAND[p]} /></div>
          {/each}
        </div>
      {:else}
        {#each cardRows as row, r (row.join())}
          <div class="row reveal" style:--i={r}>
            {#each row as p (p)}
              {@const state = snap.limits[p] ?? { state: "loading" as const }}
              {@const limits = limitsOf(state)}
              <div class="cell">
                <ProviderBlock
                  provider={p}
                  {state}
                  primary={limits ? cardWindow(snap, limits) : null}
                  size={sizeOf.get(p) ?? "m"}
                  color={snap.settings.cardColors[p] ?? BRAND[p]}
                />
              </div>
            {/each}
          </div>
        {/each}
        {#each issues as p (p)}
          <div class="reveal" style:--i={cardRows.length}>
            <IssueRow provider={p} state={snap.limits[p] ?? { state: "loading" }} />
          </div>
        {/each}
      {/if}

      {#if spendShown}
        <div class="reveal" style:--i={cardRows.length + 1}>
          {#if snap.hasUsage}
            <SpendBlock usage={snap.usage} error={snap.usageError} period={snap.period} onPeriod={choosePeriod} />
          {:else}
            <div class="block importing">
              <span>Importing usage from Claude Code and Codex logs…</span>
              <div class="progress"><div style:width="{snap.importPercent ?? 0}%"></div></div>
            </div>
          {/if}
        </div>
      {/if}

      <footer class="footer reveal" style:--i={cardRows.length + 2}>
        <span class="status" class:hidden={!snap.hasUsage}>{snap.syncing ? "Syncing…" : synced(snap.lastSync, clock.now)}</span>
      </footer>
    </div>
  {/if}
  <Tip />
</div>

<style>
  /* Room around the blocks for their shadows (Bento.shadowInset). */
  .root {
    width: 332px;
    padding: 6px 14px 18px;
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
  .reveal {
    opacity: 0;
    transform: translateY(6px);
  }
  .visible .reveal {
    opacity: 1;
    transform: none;
    transition:
      opacity 220ms var(--ease-out),
      transform 220ms var(--ease-out);
    transition-delay: calc(var(--i, 0) * 30ms);
  }
  .row {
    display: flex;
    gap: var(--gap);
  }
  .cell {
    flex: 1;
    min-width: 0;
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
    display: flex;
    align-items: center;
    gap: var(--gap);
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
  .status {
    color: var(--secondary);
  }
  .status.hidden {
    opacity: 0;
  }
</style>
