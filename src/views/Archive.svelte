<script lang="ts">
  import AreaDot from "../components/AreaDot.svelte";
  import LoadError from "../components/LoadError.svelte";
  import { t } from "../lib/copy";
  import { areas, inArea } from "../lib/areas.svelte";
  import { chatContext } from "../lib/chat.svelte";
  import { openItem } from "../lib/detail.svelte";
  import { archivedItems } from "../lib/items.svelte";
  import { poll } from "../lib/poll";
  import { dateClock } from "../lib/time";

  const { state: list, load } = archivedItems();

  $effect(() => poll(load));

  const shown = $derived(list.items.filter(inArea));

  $effect(() => {
    chatContext.text = JSON.stringify({ area: areas.current, archived: shown });
  });
</script>

<main>
  <p class="hint intro">{t("archive.hint")}</p>
  <LoadError error={list.error} />
  <ul class="list">
    {#each shown as item (`${item.node ?? ""}|${item.id}`)}
      <li>
        <AreaDot area={item.area} />
        <span class="body">
          <button type="button" class="open" onclick={() => openItem(item.id, item.node)}>
            <span class="mono id">{item.id}</span>&nbsp; {item.title}
          </button>
          {#if item.stateLabel}<span class="hint note">{item.stateLabel}</span>{/if}
        </span>
        <span class="hint when">{t("archive.doneOn", { at: dateClock(item.stateSince) })}</span>
      </li>
    {:else}
      <li class="hint empty">{t("archive.empty")}</li>
    {/each}
  </ul>
</main>

<style>
  main {
    max-width: var(--wide);
    margin: 0 auto;
    padding: 32px 24px 96px;
  }

  .intro {
    margin-bottom: 20px;
  }

  .list {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    overflow: hidden;
  }

  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--line-soft);
  }

  li:first-child {
    border-top: 0;
  }

  .body {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .open {
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .id {
    font-size: 12px;
    color: var(--muted);
  }

  .note,
  .when {
    font-size: 13px;
  }

  .when {
    flex: none;
  }

  .empty {
    padding: 20px 16px;
  }
</style>
