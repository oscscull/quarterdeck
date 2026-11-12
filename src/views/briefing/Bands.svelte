<script lang="ts">
  import { BANDS, findRef, numbering, type Band, type Placed } from "../../lib/briefing";
  import { t } from "../../lib/copy";
  import type { BriefingView } from "../../lib/types";
  import Para from "./Para.svelte";
  import RefCard from "./RefCard.svelte";

  type Tag = { name: string; color: string; onOpen: () => void };

  interface Props {
    bands: Record<Band, Placed[]>;
    view: BriefingView;
    now: Date;
    /** The project tag for a paragraph, in the combined view. */
    tagFor?: (p: Placed) => Tag | undefined;
  }

  let { bands, view, now, tagFor }: Props = $props();

  // One citation open at a time, shown under its paragraph.
  let open = $state<{ key: string; id: number; node: string | undefined; at: string } | null>(null);

  const numbers = $derived(numbering(BANDS.flatMap((b) => bands[b])));

  function toggle(p: Placed, at: string, key: string, id: number): void {
    open = open?.key === key && open.at === at ? null : { key, id, node: p.node, at };
  }
</script>

{#each BANDS as band (band)}
  <section class:done={band === "done"} aria-label={t(`briefing.band.${band}`)}>
    <h2 class="section-label">{t(`briefing.band.${band}`)}</h2>
    {#if bands[band].length === 0}
      <p class="hint">{t("briefing.bandEmpty")}</p>
    {/if}
    {#each bands[band] as p, i (`${band}${i}`)}
      {@const at = `${band}${i}`}
      {@const ref = open?.at === at ? findRef(view, open.node, open.id) : undefined}
      <Para
        paragraph={p}
        {numbers}
        open={open?.at === at ? open.key : null}
        tag={tagFor?.(p)}
        onCite={(key, id) => toggle(p, at, key, id)}
      />
      {#if ref && open}
        <RefCard reference={ref} n={numbers.get(open.key) ?? 0} {now} onClose={() => (open = null)} />
      {/if}
    {/each}
  </section>
{/each}

<style>
  section {
    margin-bottom: 30px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  h2 {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .done :global(p) {
    color: var(--muted);
  }
</style>
