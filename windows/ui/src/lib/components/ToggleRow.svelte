<!-- A settings row that is one switch: the whole row toggles. -->
<script lang="ts">
  import Switch from "./Switch.svelte";

  let { label, checked, onChange }: { label: string; checked: boolean; onChange: (on: boolean) => void } = $props();
</script>

<button class="row" role="switch" aria-checked={checked} onclick={() => onChange(!checked)}>
  <span class="label">{label}</span>
  <Switch on={checked} />
</button>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 40px;
    text-align: left;
  }
  .label {
    flex: 1;
    min-width: 0;
  }
</style>
