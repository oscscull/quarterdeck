<script lang="ts">
  import { openItem } from "../../lib/detail.svelte";
  import { t } from "../../lib/copy";
  import { turnText } from "../../lib/turn";
  import type { WorkItem } from "../../lib/types";

  interface Props {
    items: WorkItem[];
  }

  let { items }: Props = $props();
</script>

<section aria-label={t("briefing.items", { n: items.length })}>
  <h2 class="section-label">{t("briefing.items", { n: items.length })}</h2>
  {#if items.length === 0}
    <p class="hint">{t("briefing.noItems")}</p>
  {/if}
  <ul>
    {#each items as item (item.id)}
      <li>
        <button type="button" class="row-button" class:done={item.stage === "done"} onclick={() => openItem(item.id, item.node)}>
          <span class="mono id">{item.id}</span>
          <span class="title">{item.title}</span>
          <span class="stage">{t(`stage.${item.stage}`)}</span>
          {#if item.stage !== "done"}<span class="turn">{turnText(item)}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
</section>

<style>
  section {
    margin-top: 8px;
  }

  ul {
    margin-top: 6px;
  }

  .row-button {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 12px;
    padding: 10px 0;
    border: 0;
    border-top: 1px solid var(--line-soft);
    background: none;
    text-align: left;
    font-size: 14px;
    color: var(--ink);
  }

  .done {
    color: var(--muted);
  }

  .id {
    min-width: 110px;
    font-size: 12px;
    color: var(--muted);
  }

  .title {
    flex: 1 1 200px;
    min-width: 0;
  }

  .stage,
  .turn {
    font-size: 12px;
    color: var(--muted);
  }
</style>
