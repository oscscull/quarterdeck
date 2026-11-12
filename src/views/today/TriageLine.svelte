<script lang="ts">
  import AreaDot from "../../components/AreaDot.svelte";
  import { t } from "../../lib/copy";
  import { openItem } from "../../lib/detail.svelte";
  import type { Triage } from "../../lib/types";

  interface Props {
    issues: Triage[];
  }

  let { issues }: Props = $props();
  let open = $state(false);

  const label = $derived(issues.length === 1 ? t("today.triageOne") : t("today.triage", { n: issues.length }));
</script>

{#if issues.length > 0}
  <button type="button" class="link" aria-expanded={open} onclick={() => (open = !open)}>
    {open ? t("today.triageHide") : label}
  </button>
  {#if open}
    <ul>
      {#each issues as issue (`${issue.node ?? ""}${issue.itemId}`)}
        <li>
          <AreaDot area={issue.area} />
          <button type="button" class="open" onclick={() => openItem(issue.itemId, issue.node)}>
            <span class="mono id">{issue.itemId}</span>
            <span>{issue.title}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .link {
    padding: 8px 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 13px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .open {
    display: flex;
    gap: 8px;
    padding: 4px 0;
    border: 0;
    background: transparent;
    text-align: left;
    font-size: 14px;
  }

  .id {
    color: var(--muted);
    font-size: 12px;
  }
</style>
