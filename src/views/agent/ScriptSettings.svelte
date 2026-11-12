<script lang="ts">
  import { t } from "../../lib/copy";
  import type { Report, Script } from "../../lib/types";

  interface Props {
    script: Script;
    onChange: (script: Script) => void;
  }

  let { script, onChange }: Props = $props();

  const REPORTS: Report[] = ["failures", "always", "never"];

  const set = (patch: Partial<Script>) => onChange({ ...script, ...patch });
</script>

<div class="row">
  <label class="field">
    {t("script.cwd")}
    <input
      class="input mono"
      placeholder="~"
      value={script.cwd ?? ""}
      oninput={(e) => set({ cwd: e.currentTarget.value.trim() || null })}
      spellcheck="false"
    />
  </label>
  <label class="field">
    {t("script.timeout")}
    <input
      class="input"
      type="number"
      min="1"
      value={script.timeout_minutes}
      oninput={(e) => set({ timeout_minutes: Math.max(1, Math.round(Number(e.currentTarget.value) || 1)) })}
    />
  </label>
  <label class="field">
    {t("script.report")}
    <select class="input" value={script.report} onchange={(e) => set({ report: e.currentTarget.value as Report })}>
      {#each REPORTS as r (r)}<option value={r}>{t(`script.report.${r}`)}</option>{/each}
    </select>
  </label>
</div>
<p class="hint small">{t("script.hint")}</p>

<style>
  .row {
    display: grid;
    grid-template-columns: 2fr 1fr 1.4fr;
    gap: 12px;
  }

  .small {
    font-size: 12px;
  }
</style>
