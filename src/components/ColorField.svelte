<script lang="ts">
  import { contextPalette, normalizeColor } from '../lib/contexts';
  let { value = $bindable(), disabled = false }: { value: string; disabled?: boolean } = $props();
</script>

<fieldset class="color-field" {disabled}>
  <legend>Context color</legend>
  <div class="palette">
    {#each contextPalette as preset}<button
        type="button"
        class="swatch"
        class:selected={normalizeColor(value) === preset.color}
        style:background={preset.color}
        title={preset.name}
        aria-label={preset.name}
        aria-pressed={normalizeColor(value) === preset.color}
        onclick={() => (value = preset.color)}
      ></button>{/each}
  </div>
  <label
    >HEX color<input
      bind:value
      placeholder="#RRGGBB"
      spellcheck="false"
      maxlength="10"
      aria-invalid={!normalizeColor(value)}
    /></label
  >
  {#if !normalizeColor(value)}<p class="error">Use three or six HEX digits.</p>{/if}
</fieldset>

<style>
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
  }
  legend {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 12px;
  }
  .palette {
    display: grid;
    grid-template-columns: repeat(6, 32px);
    gap: 8px;
    margin-bottom: 16px;
  }
  .swatch {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    padding: 0;
    border: 2px solid transparent;
  }
  .swatch.selected {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
</style>
