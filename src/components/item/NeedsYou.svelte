<script lang="ts">
  import { t } from "../../lib/copy";
  import type { Ask, WorkItem } from "../../lib/types";
  import TellClaude from "../TellClaude.svelte";
  import AskList from "./AskList.svelte";

  interface Props {
    item: WorkItem;
    node: string | undefined;
    asks: Ask[];
  }

  let { item, node, asks }: Props = $props();

  const yours = $derived(item.turn === "you" && item.stage !== "done");
  // A question asked on the item already says what's needed; the need repeats it.
  const need = $derived(yours && asks.length === 0 ? (item.need ?? item.stateLabel) : "");
</script>

{#if need}
  <section class="needs" aria-label={t("item.neededFromYou")}>
    <h3 class="section-label">{t("item.neededFromYou")}</h3>
    <p class="need">{need}</p>
  </section>
{/if}
<AskList {node} {asks} />
{#if item.stage !== "done"}
  <section class="tell" aria-label={yours ? t("item.reply") : t("tell.label")}>
    <h3 class="section-label">{yours ? t("item.reply") : t("tell.label")}</h3>
    <TellClaude itemId={item.id} {node} />
  </section>
{/if}

<style>
  .needs {
    padding: 14px 16px;
    border-radius: 12px;
    background: var(--needs-bg);
  }

  .need {
    margin-top: 4px;
    font-size: 17px;
    line-height: 1.45;
  }

  .tell {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
