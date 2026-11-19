<script lang="ts">
  import { t } from "../lib/copy";
  import { PRIMARY_VIEWS, SECONDARY_VIEWS, viewLabel, type ViewId } from "../lib/views";

  interface Props {
    view: ViewId;
    onSelect: (view: ViewId) => void;
  }

  let { view, onSelect }: Props = $props();
</script>

{#snippet tab(id: ViewId)}
  <button
    type="button"
    class:active={id === view}
    aria-current={id === view ? "page" : undefined}
    onclick={() => onSelect(id)}
  >
    {t(viewLabel(id))}
  </button>
{/snippet}

<nav aria-label="Views">
  <div class="group">
    {#each PRIMARY_VIEWS as id (id)}{@render tab(id)}{/each}
  </div>
  <div class="group secondary">
    {#each SECONDARY_VIEWS as id (id)}{@render tab(id)}{/each}
  </div>
</nav>

<style>
  nav {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
  }

  .group {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    background: var(--switch-track);
  }

  button {
    padding: 8px 16px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font-size: 14px;
  }

  .secondary {
    opacity: 0.75;
  }

  .secondary button {
    padding: 6px 12px;
    font-size: 13px;
  }

  .secondary:hover,
  .secondary:focus-within,
  .secondary:has(.active) {
    opacity: 1;
  }

  .active {
    background: var(--surface);
    color: var(--ink);
    font-weight: 500;
  }
</style>
