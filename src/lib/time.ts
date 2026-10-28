import { t } from "./copy";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Items open longer than this are shown as stale. */
export const STALE_AFTER = 3 * DAY;

const clockFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
const weekdayFormat = new Intl.DateTimeFormat(undefined, { weekday: "short" });
const longDateFormat = new Intl.DateTimeFormat(undefined, {
  weekday: "long",
  day: "numeric",
  month: "long",
});

export const clock = (iso: string): string => clockFormat.format(new Date(iso));
export const longDate = (date: Date): string => longDateFormat.format(date);

const sameDay = (a: Date, b: Date): boolean => a.toDateString() === b.toDateString();

/** "16:00" today, otherwise "Thu 09:30". */
export function dayClock(iso: string, now: Date): string {
  const date = new Date(iso);
  return sameDay(date, now) ? clock(iso) : `${weekdayFormat.format(date)} ${clock(iso)}`;
}

/** "in 28 min", "in 2h 5m", "in 9d 4h", or "now" once started. */
export function until(iso: string, now: Date): string {
  const ms = new Date(iso).getTime() - now.getTime();
  if (ms <= 0) return t("time.now");
  const minutes = Math.ceil(ms / MINUTE);
  if (minutes < 60) return t("time.inMinutes", { n: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("time.inHours", { h: hours, m: minutes % 60 });
  return t("time.inDays", { d: Math.floor(hours / 24), h: hours % 24 });
}

const dateClockFormat = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});

/** "Thu 12 Nov, 09:00". */
export const dateClock = (iso: string): string => dateClockFormat.format(new Date(iso));

export function ageMs(iso: string, now: Date): number {
  return now.getTime() - new Date(iso).getTime();
}

/** Compact age: "42m", "2h", "9d". */
export function age(iso: string, now: Date): string {
  const ms = Math.max(0, ageMs(iso, now));
  if (ms < HOUR) return t("age.minutes", { n: Math.floor(ms / MINUTE) });
  if (ms < DAY) return t("age.hours", { n: Math.floor(ms / HOUR) });
  return t("age.days", { n: Math.floor(ms / DAY) });
}

export const isStale = (iso: string, now: Date): boolean => ageMs(iso, now) >= STALE_AFTER;

const shortDateFormat = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" });

/** "overdue", "today", "tomorrow", "Fri" within a week, otherwise "3 Nov". */
export function dueText(date: string, now: Date): string {
  const due = new Date(`${date}T00:00:00`);
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const days = Math.round((due.getTime() - today.getTime()) / DAY);
  if (days < 0) return t("todo.overdue");
  if (days === 0) return t("todo.today");
  if (days === 1) return t("todo.tomorrow");
  if (days < 7) return weekdayFormat.format(due);
  return shortDateFormat.format(due);
}

export const isOverdue = (date: string, now: Date): boolean =>
  new Date(`${date}T23:59:59`).getTime() < now.getTime();
