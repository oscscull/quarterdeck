<script lang="ts">
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/copy";
  import { followSync, github } from "../../lib/sync.svelte";

  let note = $state("");

  $effect(() => followSync());

  async function sync(): Promise<void> {
    try {
      note = (await api.syncGithub()) ? "" : t("today.syncOff");
    } catch (e) {
      note = t("today.syncFailed", { error: errorText(e) });
    }
  }
</script>

<div class="sync">
  {#if note}<span class="hint note">{note}</span>{/if}
  <button type="button" class="link" disabled={github.syncing} aria-busy={github.syncing} onclick={sync}>
    {#if github.syncing}<span class="spinner" aria-hidden="true"></span>{t("today.syncing")}{:else}{t("today.sync")}{/if}
  </button>
</div>

<style>
  .sync {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .link {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 13px;
  }

  .link:disabled {
    color: var(--muted);
    cursor: default;
  }

  .note {
    font-size: 13px;
  }

  .spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid var(--line);
    border-top-color: var(--muted);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
