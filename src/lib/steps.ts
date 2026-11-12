import { t, type CopyKey } from "./copy";
import type { LiveRun, Step } from "./types";

/** Claude Code tools with their own wording. Bash's detail is already a description. */
const TOOL_KEYS: Record<string, CopyKey> = {
  Read: "step.reading",
  Edit: "step.editing",
  MultiEdit: "step.editing",
  Write: "step.editing",
  NotebookEdit: "step.editing",
  Grep: "step.searching",
  Glob: "step.searching",
  WebFetch: "step.lookingUp",
  WebSearch: "step.lookingUp",
  Task: "step.delegating",
  Agent: "step.delegating",
};

function toolText(tool: string, detail: string): string {
  if (!detail) return t("step.using", { tool });
  if (tool === "Bash") return detail;
  const key = TOOL_KEYS[tool];
  return key ? t(key, { detail }) : t("step.tool", { tool, detail });
}

/** One step as a short line, e.g. "Reading runner.rs". */
export function stepText(step: Step): string {
  switch (step.kind) {
    case "tool":
      return toolText(step.tool ?? "", step.text);
    case "failed":
      return step.text ? t("step.failed", { text: step.text }) : t("step.failedBare");
    case "doing":
    case "said":
    case "output":
      return step.text;
  }
}

/** What a run is about: its item's title when it has one. */
export const runName = (run: LiveRun): string => run.title ?? run.what;

/** The line under a run's name: what kind of job, on what, on which machine. */
export function runMeta(run: LiveRun): string {
  if (run.title) return t("agent.runMeta", { what: run.what, target: run.target, node: run.node });
  return run.target ? t("agent.target", { target: run.target, node: run.node }) : run.node;
}

/** Runs on different machines can share an id. */
export const runKey = (run: LiveRun): string => `${run.node}:${run.id}`;
