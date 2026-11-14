<!-- Compact segmented pills whose selection slides between options (Bento.swift `PillSegmented`). -->
<script lang="ts" generics="T extends string">
  let {
    options,
    selected,
    label,
    onSelect,
    ariaLabel,
  }: { options: T[]; selected: T; label: (v: T) => string; onSelect: (v: T) => void; ariaLabel: string } = $props();

  let buttons: HTMLButtonElement[] = $state([]);
  let thumb = $state<{ left: number; width: number } | null>(null);
  // Slides only after the first placement, so it doesn't fly in when the panel opens.
  let ready = $state(false);

  // Measured after the DOM update: the selected label is bold, so its width changes with it.
  $effect(() => {
    const el = buttons[options.indexOf(selected)];
    if (!el) return;
    thumb = { left: el.offsetLeft, width: el.offsetWidth };
    requestAnimationFrame(() => (ready = true));
  });
</script>

<div class="track" role="radiogroup" aria-label={ariaLabel}>
  {#if thumb}
    <span class="thumb" class:ready style:transform="translateX({thumb.left}px)" style:width="{thumb.width}px"></span>
  {/if}
  {#each options as o, i (o)}
    <button
      bind:this={buttons[i]}
      class:selected={o === selected}
      role="radio"
      aria-checked={o === selected}
      onclick={() => onSelect(o)}>{label(o)}</button
    >
  {/each}
</div>

<style>
  .track {
    position: relative;
    display: flex;
    padding: 2px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--lime-ink) 8%, transparent);
  }
  .thumb {
    position: absolute;
    top: 2px;
    left: 0;
    height: 22px;
    border-radius: 999px;
    background: var(--lime-ink);
  }
  .thumb.ready {
    transition:
      transform var(--snappy),
      width var(--snappy);
  }
  button {
    position: relative;
    height: 22px;
    padding: 0 8px;
    border-radius: 999px;
    font-size: 11px;
    color: color-mix(in srgb, var(--lime-ink) 70%, transparent);
    transition: color 150ms ease-out;
  }
  button.selected {
    color: var(--lime);
    font-weight: 700;
  }
</style>
