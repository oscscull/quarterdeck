use schemars::JsonSchema;
use serde::Deserialize;

use super::super::ctx::Ctx;
use super::super::tool::{Tool, async_tool};
use super::in_area;
use crate::claude::sessions;
use crate::domain::ask::Ask;
use crate::domain::session::SessionRef;
use crate::domain::turn::Turn;
use crate::error::AppResult;
use crate::store::{asks, items};
use crate::turns::{self, Clock, Pass};

use crate::priorities::{Priority, priorities};

pub fn tools() -> Vec<Tool> {
    vec![
        async_tool(
            "list_asks",
            "Everything that needs the user, in item rank order: questions raised on an item \
             (kind `ask`), else the item itself when it's their turn (kind `yours`, with `need`: \
             what's wanted from them). Today shows the first few.",
            list,
        ),
        async_tool(
            "add_ask",
            "Asks the user something about an item: a design fork, a question you can't answer. \
             It makes the item the user's turn, with the question \
             as what's needed, and appears on Today if the item ranks high enough. Put the question in one \
             line and the background in `context`. Offer `options` when the answer is a choice \
             (e.g. [\"Yes\", \"No\"]); they can always write an answer instead. Quarterdeck resumes \
             your session with the answer; don't wait or poll for it. Answers are also kept on the item's history.",
            add,
        ),
        async_tool(
            "answer_ask",
            "Answers an ask on the user's behalf, when they told you the answer. Returns \"gone\" \
             if it was already answered or dismissed, and changes nothing.",
            answer,
        ),
        async_tool(
            "tell_claude",
            "Passes the user's instruction about an item to Claude, e.g. \"merge develop in\". Quarterdeck \
             resumes the session working on it, or starts one in the repo's project (for an item \
             linking a GitHub issue: on a new branch, ending in a PR Quarterdeck opens). Use when the \
             user tells you what should happen to an item that you won't do yourself in this session.",
            tell,
        ),
        async_tool(
            "dismiss_ask",
            "Removes an ask that no longer matters, without answering it.",
            dismiss,
        ),
    ]
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListArgs {
    /// Only this area, by id.
    area: Option<String>,
}

async fn list(ctx: Ctx, args: ListArgs) -> AppResult<Vec<Priority>> {
    let ranked = items::list(&ctx.db).await?;
    let mut all = priorities(&ranked, &asks::list(&ctx.db).await?);
    all.retain(|p| in_area(args.area.as_ref(), &p.area));
    Ok(all)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddArgs {
    item_id: String,
    question: String,
    /// Where it came from or why it matters, e.g. "Design fork on PR #812".
    #[serde(default)]
    context: String,
    /// Choices to answer with, e.g. `["Yes", "No"]`. Leave out for a written answer.
    #[serde(default)]
    options: Vec<String>,
    /// Usually left out: the answer resumes your Claude Code session, which Quarterdeck reads
    /// itself. A session given must exist on this machine.
    session: Option<SessionRef>,
}

async fn add(ctx: Ctx, args: AddArgs) -> AppResult<()> {
    let ask = Ask {
        id: 0,
        item_id: args.item_id,
        question: args.question,
        context: args.context,
        options: args.options,
        asker: sessions::resolve(args.session)?,
    };
    items::get(&ctx.db, &ask.item_id).await?;
    let limits = ctx.settings()?.turns;
    let why = format!("asked: {}", ask.question);
    let pass = Pass::to(Turn::You, why).needing(ask.question.clone());
    turns::pass(&ctx.db, &ask.item_id, pass, Clock::now(&limits)).await?;
    asks::insert(&ctx.db, &ask).await.map(drop)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnswerArgs {
    ask_id: i64,
    answer: String,
}

async fn answer(ctx: Ctx, args: AnswerArgs) -> AppResult<crate::answer::Outcome> {
    let settings = ctx.settings()?;
    crate::answer::give(&ctx.db, &settings, args.ask_id, &args.answer).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AskIdArgs {
    ask_id: i64,
}

async fn dismiss(ctx: Ctx, args: AskIdArgs) -> AppResult<()> {
    asks::remove(&ctx.db, args.ask_id).await
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TellArgs {
    item_id: String,
    text: String,
}

async fn tell(ctx: Ctx, args: TellArgs) -> AppResult<()> {
    let settings = ctx.settings()?;
    crate::answer::tell(&ctx.db, &settings, &args.item_id, &args.text).await
}
