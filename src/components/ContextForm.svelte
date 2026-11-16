<script lang="ts">
  import { untrack } from 'svelte';
  import Modal from './Modal.svelte';
  import ColorField from './ColorField.svelte';
  import type { Controller } from '../lib/controller.svelte';
  import { errorMessage } from '../lib/types';
  import { contextRows, normalizeColor, suggestContextColor } from '../lib/contexts';
  let {
    app,
    parentId = null,
    onclose,
  }: { app: Controller; parentId?: string | null; onclose: () => void } = $props();
  let name = $state('');
  let parent = $state(untrack(() => parentId ?? ''));
  let color = $state(untrack(() => suggestContextColor(app.contexts, parentId)));
  let error = $state('');
  let busy = $state(false);
  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    busy = true;
    error = '';
    try {
      const receipt = await app.change({
        type: 'create_context',
        name,
        parentId: parent || null,
        color,
      });
      app.navigate({ kind: 'context', id: receipt.id });
      onclose();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<Modal title="New context" {busy} {onclose}>
  <form onsubmit={save}>
    <label
      >Name<input
        bind:value={name}
        maxlength="240"
        autocomplete="off"
        disabled={busy}
        required
      /></label
    >
    <label
      >Parent context<select
        bind:value={parent}
        disabled={busy}
        onchange={() => {
          color = suggestContextColor(app.contexts, parent || null);
        }}
        ><option value="">No parent</option>{#each contextRows(app.contexts) as row}<option
            value={row.id}>{row.path}</option
          >{/each}</select
      ></label
    >
    <ColorField bind:value={color} disabled={busy} />
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="dialog-actions">
      <button type="button" disabled={busy} onclick={onclose}>Cancel</button><button
        class="primary"
        disabled={busy || !name.trim() || !normalizeColor(color)}>Create</button
      >
    </div>
  </form>
</Modal>
