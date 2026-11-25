<script lang="ts">
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/copy";
  import { followSync, github } from "../../lib/sync.svelte";

  let note = $state("");

  $effect(() => followSync());

  async function sync(): Promise<void> {
    try {
      note = (await api.syncGithub()) ? "" : t("priorities.syncOff");
    } catch (e) {
      note = t("priorities.syncFailed", { error: errorText(e) });
    }
  }
</script>

<div class="sync">
  <button type="button" class="button-secondary" disabled={github.syncing} aria-busy={github.syncing} onclick={sync}>
    {#if github.syncing}<span class="spinner" aria-hidden="true"></span>{t("priorities.syncing")}{:else}{t("priorities.sync")}{/if}
  </button>
  {#if note}<p class="hint note">{note}</p>{/if}
</div>

<style>
  .sync {
    margin-top: 16px;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  button {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .note {
    font-size: 13px;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--line);
    border-top-color: var(--ink);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
