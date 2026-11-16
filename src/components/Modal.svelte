<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  let {
    title,
    busy = false,
    onclose,
    children,
  }: { title: string; busy?: boolean; onclose: () => void; children: Snippet } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => {
    const opener = document.activeElement;
    dialog.showModal();
    return () => {
      dialog.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  });
</script>

<!-- Primary: dialog task. Secondary: cancel/close. Shared surface, 24px spacing,
     14/18px type, large radius and dialog elevation follow the existing tokens. -->
<dialog
  bind:this={dialog}
  aria-label={title}
  {onclose}
  oncancel={(event) => {
    if (busy) event.preventDefault();
  }}
>
  <div class="dialog-heading">
    <h2>{title}</h2>
    <button type="button" class="icon-button" aria-label="Close" disabled={busy} onclick={onclose}
      >×</button
    >
  </div>
  {@render children()}
</dialog>
