<!-- Round icon button (Bento.swift `RoundIconButton`). -->
<script lang="ts">
  import { tip } from "../tip.svelte";
  import Icons from "./Icons.svelte";

  let {
    icon,
    label,
    onclick,
    size = 28,
  }: { icon: "gear" | "chevron-left"; label: string; onclick: () => void; size?: number } = $props();
</script>

<button class="round" style:width="{size}px" style:height="{size}px" aria-label={label} use:tip={label} {onclick}>
  <Icons name={icon} size={Math.round(size * 0.46)} />
</button>

<style>
  .round {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    border-radius: 50%;
    background: var(--neutral);
    color: var(--secondary);
    transition:
      background 150ms ease-out,
      color 150ms ease-out,
      transform 120ms ease-out;
  }
  /* Hit area of 40 px. */
  .round::after {
    content: "";
    position: absolute;
    inset: -6px;
  }
  .round:hover {
    background: var(--neutral-hover);
    color: var(--text);
  }
  .round:active {
    transform: scale(0.96);
  }
</style>
