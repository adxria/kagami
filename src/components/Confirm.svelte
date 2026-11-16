<script lang="ts">
  import Modal from './Modal.svelte';
  import { errorMessage } from '../lib/types';
  let {
    title,
    message,
    label,
    run,
    onclose,
  }: {
    title: string;
    message: string;
    label: string;
    run: () => Promise<unknown>;
    onclose: () => void;
  } = $props();
  let busy = $state(false);
  let error = $state('');
  async function confirm() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await run();
      onclose();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<Modal {title} {busy} {onclose}>
  <p>{message}</p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="dialog-actions">
    <button disabled={busy} onclick={onclose}>Cancel</button><button
      class="danger"
      disabled={busy}
      onclick={confirm}>{label}</button
    >
  </div>
</Modal>
