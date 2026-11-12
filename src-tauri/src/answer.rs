//! The user answering an ask, or telling the agent what to do about an item. Either way it stays
//! on the item's history, and the turn passes at once.

use chrono::Utc;
use serde::Serialize;

use crate::config::settings::Settings;
use crate::domain::ask::Ask;
use crate::domain::event::Change;
use crate::domain::turn::Turn;
use crate::error::{AppError, AppResult};
use crate::store::{Db, asks, events, items};
use crate::turns::{self, Clock, Pass};

fn nonempty(text: &str) -> AppResult<&str> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::BadInput("that can't be empty".to_owned()));
    }
    Ok(text)
}

/// What came of a reply. A screen can show a question after it was answered or dismissed
/// elsewhere, so answering one that's gone is a normal outcome, not an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Done,
    /// The question was already answered or dismissed; nothing changed.
    Gone,
}

pub async fn give(db: &Db, settings: &Settings, ask_id: i64, answer: &str) -> AppResult<Outcome> {
    let answer = nonempty(answer)?;
    let Some(ask) = asks::find_open(db, ask_id).await? else {
        return Ok(Outcome::Gone);
    };
    let text = format!("You answered “{answer}” to: {}", ask.question);
    events::record(db, &ask.item_id, Utc::now(), &Change::Note { text }).await?;
    asks::answer(db, ask_id, answer).await?;
    hand_over(db, settings, &ask, format!("you answered: {answer}")).await?;
    Ok(Outcome::Done)
}

/// A message about an item with no question behind it. Stored as an already-answered ask with no
/// question, so it reaches Claude the same way an answer does.
pub async fn tell(db: &Db, settings: &Settings, item_id: &str, message: &str) -> AppResult<()> {
    let message = nonempty(message)?;
    items::get(db, item_id).await?;
    let ask = Ask {
        item_id: item_id.to_owned(),
        ..Ask::default()
    };
    let id = asks::insert(db, &ask).await?;
    let text = format!("You told Claude: {message}");
    events::record(db, item_id, Utc::now(), &Change::Note { text }).await?;
    asks::answer(db, id, message).await?;
    hand_over(db, settings, &ask, format!("you said: {message}")).await
}

/// A reply ends the user's turn: the agent has it, unless another question on the item is open.
async fn hand_over(db: &Db, settings: &Settings, ask: &Ask, why: String) -> AppResult<()> {
    if !asks::for_item(db, &ask.item_id).await?.is_empty() {
        return Ok(());
    }
    let pass = Pass::to(Turn::Agent, why);
    turns::pass(db, &ask.item_id, pass, Clock::now(&settings.turns)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{ItemInput, Stage};
    use crate::work;

    async fn decision(db: &Db, id: &str) {
        let input = ItemInput {
            id: id.to_owned(),
            title: "t".to_owned(),
            state_label: "your call".to_owned(),
            stage: Stage::InReview,
            status: None,
            area: "work".to_owned(),
            link: None,
        };
        work::save(db, input, Utc::now()).await.unwrap();
    }

    async fn ask(db: &Db, item: &str) -> i64 {
        let ask = Ask {
            item_id: item.to_owned(),
            question: "Merge?".to_owned(),
            ..Ask::default()
        };
        asks::insert(db, &ask).await.unwrap()
    }

    #[tokio::test]
    async fn a_reply_gives_the_agent_the_turn_once_nothing_else_is_asked() {
        let db = Db::in_memory().await;
        let settings = Settings::default();
        decision(&db, "a").await;
        let first = ask(&db, "a").await;
        let second = ask(&db, "a").await;
        give(&db, &settings, first, "Yes").await.unwrap();
        assert!(items::get(&db, "a").await.unwrap().is_turn(Turn::You));
        give(&db, &settings, second, "No").await.unwrap();
        assert!(items::get(&db, "a").await.unwrap().is_turn(Turn::Agent));

        decision(&db, "b").await;
        tell(&db, &settings, "b", "fix the conflict").await.unwrap();
        assert!(items::get(&db, "b").await.unwrap().is_turn(Turn::Agent));
    }

    #[tokio::test]
    async fn answering_a_question_that_is_gone_changes_nothing() {
        let db = Db::in_memory().await;
        let settings = Settings::default();
        decision(&db, "a").await;
        let answered = ask(&db, "a").await;
        let dismissed = ask(&db, "a").await;
        let first = give(&db, &settings, answered, "Yes").await.unwrap();
        assert_eq!(first, Outcome::Done);
        asks::remove(&db, dismissed).await.unwrap();
        let history = events::for_item(&db, "a").await.unwrap().len();
        let again = give(&db, &settings, answered, "No").await.unwrap();
        assert_eq!(again, Outcome::Gone);
        let after_dismiss = give(&db, &settings, dismissed, "No").await.unwrap();
        assert_eq!(after_dismiss, Outcome::Gone);
        assert_eq!(events::for_item(&db, "a").await.unwrap().len(), history);
    }
}
