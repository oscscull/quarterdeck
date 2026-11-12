<script lang="ts">
  import type { Snippet } from "svelte";

  import { t } from "../lib/copy";

  interface Props {
    title: string;
    subtitle?: string;
    /** For dialogs that carry a lot of reading, like an item's detail. */
    wide?: boolean;
    onClose: () => void;
    children: Snippet;
  }

  let { title, subtitle, wide = false, onClose, children }: Props = $props();

  function onKey(event: KeyboardEvent): void {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="dialog" class:wide role="dialog" aria-modal="true" aria-label={title} tabindex="-1">
    <div class="head">
      <div>
        <h2>{title}</h2>
        {#if subtitle}<p class="hint">{subtitle}</p>{/if}
      </div>
      <button type="button" class="close" aria-label={t("modal.close")} onclick={onClose}>×</button>
    </div>
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(25, 25, 23, 0.28);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 64px 16px;
    overflow-y: auto;
  }

  .dialog {
    width: 100%;
    max-width: 600px;
    background: var(--surface);
    border-radius: 16px;
    box-shadow: 0 20px 60px rgba(25, 25, 23, 0.18);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .dialog.wide {
    max-width: 820px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }

  h2 {
    font-size: 20px;
    font-weight: 600;
  }

  .close {
    width: 36px;
    height: 36px;
    flex: none;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font-size: 20px;
  }
</style>
