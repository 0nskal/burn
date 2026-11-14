<!-- Small filled glyphs standing in for the SF Symbols the Mac app uses. -->
<script lang="ts">
  let { name, size = 10 }: { name: "alert-circle" | "alert-triangle" | "gear" | "share" | "check" | "plus" | "close" | "chevron-left" | "chevron-right"; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true">
  {#if name === "alert-circle"}
    <mask id="ac"><rect width="16" height="16" fill="white" /><rect x="7" y="3.6" width="2" height="5.6" rx="1" fill="black" /><circle cx="8" cy="11.6" r="1.15" fill="black" /></mask>
    <circle cx="8" cy="8" r="8" fill="currentColor" mask="url(#ac)" />
  {:else if name === "alert-triangle"}
    <mask id="at"><rect width="16" height="16" fill="white" /><rect x="7" y="5" width="2" height="5" rx="1" fill="black" /><circle cx="8" cy="12.3" r="1.1" fill="black" /></mask>
    <path d="M6.6 1.6a1.6 1.6 0 0 1 2.8 0l6.3 11.2A1.6 1.6 0 0 1 14.3 15H1.7a1.6 1.6 0 0 1-1.4-2.2Z" fill="currentColor" mask="url(#at)" />
  {:else if name === "gear"}
    <path
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linejoin="round"
      d="M6.7 1.5h2.6l.4 1.9 1.3.7 1.8-.7 1.3 2.2-1.4 1.3v1.4l1.4 1.3-1.3 2.2-1.8-.7-1.3.7-.4 1.9H6.7l-.4-1.9-1.3-.7-1.8.7-1.3-2.2 1.4-1.3V7.3L1.9 6l1.3-2.2 1.8.7 1.3-.7Z"
    />
    <circle cx="8" cy="8" r="2.1" fill="none" stroke="currentColor" stroke-width="1.5" />
  {:else if name === "share"}
    <path fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" d="M8 10V1.8M5.2 4.4 8 1.6l2.8 2.8M5.5 6.5H4a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1v-6a1 1 0 0 0-1-1h-1.5" />
  {:else if name === "check"}
    <path fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="m3 8.5 3.2 3.2L13 4.8" />
  {:else if name === "plus"}
    <path fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" d="M8 3v10M3 8h10" />
  {:else if name === "close"}
    <path fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" d="m3.5 3.5 9 9m0-9-9 9" />
  {:else if name === "chevron-left"}
    <path fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="m10 3-5 5 5 5" />
  {:else if name === "chevron-right"}
    <path fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="m6 3 5 5-5 5" />
  {/if}
</svg>

<style>
  svg {
    display: block;
    flex: none;
  }
</style>
