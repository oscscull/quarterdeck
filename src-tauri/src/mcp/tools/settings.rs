use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{NoArgs, Tool, tool};
use crate::config::calendar::Feed;
use crate::config::edit;
use crate::config::settings::Area;
use crate::connectors::calendar::macos::{self, MacCalendar};
use crate::error::AppResult;

const UPSERT_AREA: &str = "Adds an area of life, or changes one by id: its name, dot colour (CSS hex) and the \
machine (`node`) that runs its schedules. Use list_areas to see machine names.";
const ADD_FEED: &str = "Adds a calendar, or replaces one with the same name. Its events become meetings in \
`area`. Give either `url`, its private iCal link (Google: Settings > the calendar > \"Secret address in iCal \
format\"), or `mac_calendar`, a calendar's title in the macOS Calendar app (see list_mac_calendars). The Calendar \
app works for accounts that offer no iCal link, once the account is added in System Settings > Internet Accounts.";
const LIST_MAC: &str = "Lists the calendars in this Mac's Calendar app, with the account each belongs to. The first \
call may wait while macOS asks the user to allow calendar access.";
const CALENDAR_RULES: &str = "Sets which calendar events Quarterdeck skips for every calendar: titles containing \
any of `ignore_titles` (case-insensitive), and optionally all-day events. To hide a single event, use \
update_meeting with hidden instead.";
const MAP_GITHUB: &str = "Files PRs and issues from a GitHub owner (e.g. \"my-org\") or repo (\"me/app\") under an area. \
A repo entry beats its owner's.";
const WATCH_ISSUES: &str = "Makes a GitHub owner's (e.g. \"me\") or repo's (\"my-org/app\") open issues \
become items, or stops it (`watch: false`). Pass `area` to file them (and that owner's or repo's PRs) under \
it. Issues come in on the normal GitHub sync with no agent; they close when GitHub closes them. Issues already \
covered by one of the user's open PRs are left to that PR's item. To drop a single issue, close or delete \
its item.";
const IGNORE_GITHUB: &str = "Sets the GitHub owners or repos whose PRs Quarterdeck skips entirely.";
const SELF_REVIEW: &str = "Sets the GitHub owners or repos where the user reviews their own PRs, so an unreviewed PR \
shows as waiting on their decision rather than on reviewers. PRs nobody has been asked to review, or has reviewed, are \
treated that way already.";

const INFORMATIONAL: &str = "Sets which checks on an owner's or repo's PRs only inform, e.g. a visual-diff \
report like \"reg\". Their failures don't count as CI failing (so Quarterdeck won't start a fix), and \
their results are noted on the item instead. An empty list clears it.";

pub fn tools() -> Vec<Tool> {
    vec![
        tool("upsert_area", UPSERT_AREA, upsert_area),
        tool("add_calendar", ADD_FEED, add_feed),
        tool("list_mac_calendars", LIST_MAC, list_mac),
        tool(
            "remove_calendar",
            "Stops syncing a calendar, by name.",
            remove_feed,
        ),
        tool("set_calendar_rules", CALENDAR_RULES, calendar_rules),
        tool("map_github_area", MAP_GITHUB, map_github),
        tool("watch_github_issues", WATCH_ISSUES, watch_issues),
        tool("set_github_ignores", IGNORE_GITHUB, github_ignores),
        tool("set_github_self_review", SELF_REVIEW, self_review),
        tool("set_informational_checks", INFORMATIONAL, informational),
    ]
}

/// An area runs on this machine or a peer it knows: one on a machine nobody knows would have its
/// schedules and connectors run nowhere.
fn upsert_area(ctx: &Ctx, area: Area) -> AppResult<()> {
    let settings = ctx.settings()?;
    if area.node != settings.node && settings.sync.peer(&area.node).is_none() {
        return Err(crate::error::AppError::BadInput(format!(
            "no machine called '{}': use this one ('{}') or a peer (add_peer first)",
            area.node, settings.node
        )));
    }
    edit::upsert_area(&ctx.config, &area)
}

fn add_feed(ctx: &Ctx, feed: Feed) -> AppResult<()> {
    let feed = Feed {
        area: ctx.area(&feed.area)?,
        ..feed
    };
    edit::upsert_feed(&ctx.config, &feed)
}

fn list_mac(_: &Ctx, _: NoArgs) -> AppResult<Vec<MacCalendar>> {
    macos::calendars()
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NameArgs {
    name: String,
}

fn remove_feed(ctx: &Ctx, args: NameArgs) -> AppResult<()> {
    edit::remove_feed(&ctx.config, &args.name)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CalendarRules {
    ignore_titles: Vec<String>,
    /// Skip all-day events (holidays, birthdays). Leave out to keep the current setting.
    ignore_all_day: Option<bool>,
}

fn calendar_rules(ctx: &Ctx, args: CalendarRules) -> AppResult<()> {
    edit::set_calendar_ignores(&ctx.config, &args.ignore_titles, args.ignore_all_day)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MapArgs {
    /// e.g. "my-org" or "me/app"
    owner_or_repo: String,
    /// Area id.
    area: String,
}

fn map_github(ctx: &Ctx, args: MapArgs) -> AppResult<()> {
    edit::map_github(&ctx.config, &args.owner_or_repo, &ctx.area(&args.area)?)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WatchArgs {
    /// e.g. "me" or "my-org/app"
    owner_or_repo: String,
    /// False stops watching. Defaults to true.
    #[serde(default = "yes")]
    watch: bool,
    /// Area id to file this owner's or repo's issues and PRs under.
    area: Option<String>,
}

const fn yes() -> bool {
    true
}

fn watch_issues(ctx: &Ctx, args: WatchArgs) -> AppResult<()> {
    if let Some(area) = &args.area {
        edit::map_github(&ctx.config, &args.owner_or_repo, &ctx.area(area)?)?;
    }
    edit::set_issue_watch(&ctx.config, &args.owner_or_repo, args.watch)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IgnoreArgs {
    ignore: Vec<String>,
}

fn github_ignores(ctx: &Ctx, args: IgnoreArgs) -> AppResult<()> {
    edit::set_github_ignores(&ctx.config, &args.ignore)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SelfReviewArgs {
    owners_or_repos: Vec<String>,
}

fn self_review(ctx: &Ctx, args: SelfReviewArgs) -> AppResult<()> {
    edit::set_github_self_review(&ctx.config, &args.owners_or_repos)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct InformationalArgs {
    /// e.g. "my-org" or "me/app"
    owner_or_repo: String,
    /// Check names as GitHub shows them, e.g. `reg`.
    checks: Vec<String>,
}

fn informational(ctx: &Ctx, args: InformationalArgs) -> AppResult<()> {
    edit::set_informational_checks(&ctx.config, &args.owner_or_repo, &args.checks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_calendar_offers_a_link_or_a_mac_calendar() {
        let add = tools()
            .into_iter()
            .find(|t| t.name == "add_calendar")
            .unwrap();
        let schema = add.schema.to_string();
        assert!(schema.contains("\"url\"") && schema.contains("\"mac_calendar\""));
    }
}
