<script lang="ts">
  let {
    name,
  }: {
    name: 'calendar' | 'lock' | 'chevron' | 'check' | 'settings' | 'focus' | 'play';
  } = $props();
</script>

<svg
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if name === 'calendar'}
    <rect x="2" y="3.5" width="12" height="10.5" rx="2" />
    <path d="M5 2v3M11 2v3M2 7h12M5 10h1M10 10h1" />
  {:else if name === 'lock'}
    <rect x="3.5" y="7" width="9" height="7" rx="1.5" />
    <path d="M5.5 7V4.5a2.5 2.5 0 0 1 5 0V7M8 10v2" />
  {:else if name === 'chevron'}
    <path d="m4 6 4 4 4-4" />
  {:else if name === 'settings'}
    <path d="M2 4h4m3 0h5M2 8h7m3 0h2M2 12h2m3 0h7M6 2.5v3m6 1v3m-8 1v3" />
  {:else if name === 'focus'}
    <path d="M5.5 2H2v3.5m8.5-3.5H14v3.5M2 10.5V14h3.5m8.5-3.5V14h-3.5" />
    <circle cx="8" cy="8" r="2" />
  {:else if name === 'play'}
    <path d="M5.5 3.5 12 8l-6.5 4.5Z" fill="currentColor" />
  {:else}
    <path d="m3 8 3 3 7-7" />
  {/if}
</svg>

<style>
  svg {
    display: inline-block;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    vertical-align: middle;
  }
</style>
