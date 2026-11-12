<script lang="ts">
  import { refKey, segments, type Placed } from "../../lib/briefing";
  import { t } from "../../lib/copy";

  interface Props {
    paragraph: Placed;
    numbers: Map<string, number>;
    /** The open citation's key, if it is in this paragraph. */
    open: string | null;
    /** The project's name and colour, shown before the text in the combined view. */
    tag?: { name: string; color: string; onOpen: () => void } | undefined;
    onCite: (key: string, id: number) => void;
  }

  let { paragraph, numbers, open, tag, onCite }: Props = $props();

  const parts = $derived(segments(paragraph.text));
</script>

<p>
  {#if tag}
    <button type="button" class="tag" onclick={tag.onOpen}><span class="dot" style:background={tag.color}></span>{tag.name}</button>
  {/if}
  {#each parts as part, i (i)}
    {#if part.kind === "text"}{part.text}{:else}
      {@const key = refKey(paragraph.node, part.id)}
      {@const n = numbers.get(key) ?? 0}
      <button
        type="button"
        class="cite"
        class:on={open === key}
        aria-expanded={open === key}
        aria-label={t("briefing.cite", { n })}
        onclick={() => onCite(key, part.id)}>{n}</button
      >
    {/if}
  {/each}
</p>

<style>
  p {
    font-size: 17px;
    line-height: 1.6;
  }

  .tag {
    margin-right: 6px;
    padding: 0;
    border: 0;
    background: none;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    color: var(--ink);
  }

  .tag:hover {
    color: var(--accent);
  }

  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    margin-right: 5px;
    border-radius: 50%;
    vertical-align: 1px;
  }

  .cite {
    margin-left: 1px;
    padding: 0 2px;
    border: 0;
    border-radius: 3px;
    background: none;
    font-size: 11px;
    font-weight: 600;
    line-height: 0;
    vertical-align: super;
    color: var(--accent);
  }

  .cite.on {
    background: var(--chip-review-bg);
  }
</style>
