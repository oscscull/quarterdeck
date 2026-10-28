<script lang="ts">
  import ItemCard from "../components/ItemCard.svelte";
  import LoadError from "../components/LoadError.svelte";
  import { t } from "../lib/copy";
  import { newItem } from "../lib/detail.svelte";
  import { STAGES } from "../lib/groups";
  import { ALL, areas, inArea } from "../lib/areas.svelte";
  import { chatContext } from "../lib/chat.svelte";
  import { workItems } from "../lib/items.svelte";
  import { poll } from "../lib/poll";

  const { state: list, load } = workItems();

  $effect(() => poll(load));

  const newArea = (): string => (areas.current !== ALL ? areas.current : (areas.list[0]?.id ?? ""));
  const isEmpty = $derived(list.items.filter(inArea).length === 0);

  const columns = $derived(
    STAGES.map((stage) => ({ stage, items: list.items.filter((i) => i.stage === stage && i.turn !== "later" && inArea(i)) })),
  );

  $effect(() => {
    chatContext.text = JSON.stringify({ area: areas.current, columns });
  });
</script>

<main>
  <div class="top">
    {#if isEmpty}<p class="hint">{t("empty.board")}</p>{:else}<span></span>{/if}
    <button type="button" class="button-primary" onclick={() => newItem(newArea())}>{t("item.new")}</button>
  </div>
  <LoadError error={list.error} />
  <div class="scroll">
    <div class="grid">
      {#each columns as col (col.stage)}
        <section aria-label={t(`stage.${col.stage}`)}>
          <h2 class="section-label">
            {t(`stage.${col.stage}`)} <span class="mono count">{col.items.length}</span>
          </h2>
          {#each col.items as item (`${item.node ?? ""}${item.id}`)}
            <ItemCard {item} now={list.now} />
          {/each}
        </section>
      {/each}
    </div>
  </div>
</main>

<style>
  main {
    max-width: var(--wide);
    margin: 0 auto;
    padding: 32px 24px 96px;
  }

  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 20px;
  }

  .scroll {
    overflow-x: auto;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 20px;
    min-width: 880px;
    align-items: start;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h2 {
    display: flex;
    gap: 8px;
    align-items: baseline;
    margin-bottom: 4px;
  }

  .count {
    font-size: 12px;
  }
</style>
