// Reading briefings: merging machines, splitting citations out of text, and numbering them.
import { tag } from "./peers.svelte";
import type { Briefing, BriefingView, Paragraph, ProjectView, Reference } from "./types";

interface Remote<T> {
  node: string;
  value: T;
}

export type Band = keyof Briefing;
export const BANDS: readonly Band[] = ["done", "now", "next"];

/** A paragraph with the machine it came from, so its citations resolve there. */
export interface Placed extends Paragraph {
  node?: string | undefined;
}

export type Segment = { kind: "text"; text: string } | { kind: "cite"; id: number };

/** Every machine's briefings in one view, each record tagged with its machine. */
export function mergeBriefing(local: BriefingView, remote: Remote<BriefingView>[]): BriefingView {
  return {
    areas: [...local.areas, ...remote.flatMap((r) => tag(r.value.areas, r.node))],
    projects: [...local.projects, ...remote.flatMap((r) => tag(r.value.projects, r.node))],
    references: [...local.references, ...remote.flatMap((r) => tag(r.value.references, r.node))],
    writing: [...local.writing, ...remote.flatMap((r) => r.value.writing)],
  };
}

/** Splits `[ref:<id>]` markers out of a paragraph's text. */
export function segments(text: string): Segment[] {
  const out: Segment[] = [];
  const marker = /\[ref:\s*(\d+)\]/g;
  let last = 0;
  for (const m of text.matchAll(marker)) {
    const at = m.index ?? 0;
    if (at > last) out.push({ kind: "text", text: text.slice(last, at) });
    out.push({ kind: "cite", id: Number(m[1]) });
    last = at + m[0].length;
  }
  if (last < text.length) out.push({ kind: "text", text: text.slice(last) });
  return out;
}

export const refKey = (node: string | undefined, id: number): string => `${node ?? ""}:${id}`;

/** Numbers each cited reference 1, 2, 3... in reading order. */
export function numbering(paragraphs: Placed[]): Map<string, number> {
  const numbers = new Map<string, number>();
  for (const p of paragraphs) {
    for (const s of segments(p.text)) {
      const key = s.kind === "cite" ? refKey(p.node, s.id) : null;
      if (key && !numbers.has(key)) numbers.set(key, numbers.size + 1);
    }
  }
  return numbers;
}

export const findRef = (view: BriefingView, node: string | undefined, id: number): Reference | undefined =>
  view.references.find((r) => r.id === id && r.node === node);

export const projectKey = (p: { node?: string; id: string }): string => `${p.node ?? ""}:${p.id}`;

export const isActive = (p: ProjectView): boolean => p.state === "moving" || p.state === "stalled";

/** The briefing's bands with each paragraph placed on its machine. */
export function placed(briefing: Briefing | null, node: string | undefined): Record<Band, Placed[]> {
  const place = (list: Paragraph[]): Placed[] => list.map((p) => ({ ...p, node }));
  return {
    done: place(briefing?.done ?? []),
    now: place(briefing?.now ?? []),
    next: place(briefing?.next ?? []),
  };
}

/** The combined bands for several areas, in the order the areas are listed. */
export function combined(view: BriefingView, areaOrder: string[], shown: (area: string) => boolean): Record<Band, Placed[]> {
  const out: Record<Band, Placed[]> = { done: [], now: [], next: [] };
  const rank = (area: string): number => {
    const i = areaOrder.indexOf(area);
    return i < 0 ? areaOrder.length : i;
  };
  const areas = view.areas.filter((a) => shown(a.area)).sort((a, b) => rank(a.area) - rank(b.area));
  for (const a of areas) {
    const bands = placed(a.briefing, a.node);
    for (const band of BANDS) out[band].push(...bands[band]);
  }
  return out;
}
