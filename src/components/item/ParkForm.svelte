<script lang="ts">
  import { errorText } from "../../lib/api";
  import { t } from "../../lib/copy";
  import { changed } from "../../lib/refresh.svelte";
  import { route } from "../../lib/route";

  interface Props {
    id: string;
    node: string | undefined;
    onParked: () => void;
    onCancel: () => void;
  }

  let { id, node, onParked, onCancel }: Props = $props();

  const WEEK_MS = 7 * 24 * 60 * 60 * 1000;
  /** A date input's value: local `YYYY-MM-DD`. */
  const day = (d: Date): string =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

  let until = $state(day(new Date(Date.now() + WEEK_MS)));
  let why = $state("");
  let then = $state<"you" | "agent">("you");
  let error = $state("");

  async function park(): Promise<void> {
    try {
      // Comes back at the start of the chosen day.
      const at = new Date(`${until}T09:00:00`).toISOString();
      await route.park(node, id, { until: at, why: why.trim() || null, then });
      changed();
      onParked();
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<form class="park" onsubmit={(e) => { e.preventDefault(); void park(); }}>
  <p class="hint">{t("park.hint")}</p>
  <label>
    <span class="section-label">{t("park.until")}</span>
    <input type="date" bind:value={until} min={day(new Date())} required />
  </label>
  <label>
    <span class="section-label">{t("park.why")}</span>
    <input type="text" bind:value={why} placeholder={t("park.whyPlaceholder")} required={then === "agent"} />
  </label>
  <fieldset>
    <legend class="section-label">{t("park.then")}</legend>
    <label class="choice"><input type="radio" bind:group={then} value="you" /> {t("park.thenYou")}</label>
    <label class="choice"><input type="radio" bind:group={then} value="agent" /> {t("park.thenAgent")}</label>
  </fieldset>
  {#if error}<p class="error">{error}</p>{/if}
  <div class="buttons">
    <button type="button" class="button-secondary" onclick={onCancel}>{t("item.cancel")}</button>
    <button type="submit" class="button-primary">{t("item.park")}</button>
  </div>
</form>

<style>
  .park {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  fieldset {
    display: flex;
    gap: 16px;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-bottom: 4px;
  }

  .choice {
    flex-direction: row;
    align-items: center;
    gap: 6px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
