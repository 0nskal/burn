<!-- "66%" with a smaller percent sign. -->
<script lang="ts">
  let { value, size }: { value: number; size: number } = $props();
</script>

<span class="big tabular" style:font-size="{size}px" style:letter-spacing="{-size * 0.03}px">{value}<span class="sign">%</span></span>

<style>
  .big {
    font-family: var(--font-display);
    font-weight: 800;
    line-height: 1;
    white-space: nowrap;
    display: block;
  }
  .sign {
    font-size: 0.5em;
  }
</style>
