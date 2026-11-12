//! Changing one plain setting in `quarterdeck.toml` by name.

use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, tool};
use crate::config::edit;
use crate::config::keys::{SettingKey, toml_value};
use crate::error::AppResult;

const SET_SETTING: &str = "Changes one of Quarterdeck's plain settings on this machine: limits, how long \
things are kept, sync intervals, alerts, updates. Each key says what value it takes. A value that doesn't \
suit the setting is refused and nothing changes. Other settings have their own tools (areas, calendars, \
GitHub areas, ignores and issues, projects, queue, sync, turn limits).";

pub fn tools() -> Vec<Tool> {
    vec![tool("set_setting", SET_SETTING, set)]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SetArgs {
    key: SettingKey,
    /// A number, true/false, text, or a list of text, as the key says.
    value: serde_json::Value,
}

fn set(ctx: &Ctx, args: SetArgs) -> AppResult<()> {
    edit::set_value(&ctx.config, &args.key.path(), toml_value(&args.value)?)
}
