use chrono::Local;
use futures_lite::future::yield_now;
use ordered_float::OrderedFloat;
use std::sync::Arc;
use warp_core::features::FeatureFlag;
use warpui::{AppContext, SingletonEntity};

use crate::search::async_snapshot_data_source::AsyncSnapshotDataSource;
use crate::search::command_search::searcher::CommandSearchItemAction;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::{BoxFuture, DataSourceRunErrorWrapper};
use crate::settings::AISettings;
use crate::terminal;
use crate::terminal::model::session::SessionId;
use crate::terminal::HistoryEntry;

use super::rank::{self, RankInputs};
use super::HistorySearchItem;

const CHUNK_SIZE: usize = 512;

pub(crate) struct HistorySnapshot {
    commands: Arc<[Arc<HistoryEntry>]>,
    query_text: String,
    current_session_id: SessionId,
}

/// Creates an async data source for shell history commands.
#[cfg(test)]
pub fn history_data_source(
    commands: Vec<HistoryEntry>,
) -> AsyncSnapshotDataSource<HistorySnapshot, CommandSearchItemAction> {
    let commands: Arc<[Arc<HistoryEntry>]> = commands.into_iter().map(Arc::new).collect();
    AsyncSnapshotDataSource::new(
        move |query: &Query, _app: &AppContext| HistorySnapshot {
            // Historical commands are all stored as Arcs (with COW semantics and very infrequent writes),
            // so cloning the commands to pass them in to the async sort function is a negligible cost.
            commands: commands.clone(),
            query_text: query.text.clone(),
            current_session_id: SessionId::from(0),
        },
        fuzzy_match_history,
    )
}

/// The candidate list is rebuilt from `History`'s live state on every query, so a command that
/// finishes while the panel is open is scored with its up-to-date exit status.
pub(crate) fn history_data_source_for_session(
    session_id: SessionId,
) -> AsyncSnapshotDataSource<HistorySnapshot, CommandSearchItemAction> {
    AsyncSnapshotDataSource::new(
        move |query: &Query, app: &AppContext| {
            let include_agent_commands = *AISettings::as_ref(app).include_agent_commands_in_history;
            let commands: Arc<[Arc<HistoryEntry>]> = terminal::History::as_ref(app)
                .commands_shared(session_id)
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| include_agent_commands || !entry.is_agent_executed)
                .collect();
            HistorySnapshot {
                commands,
                query_text: query.text.clone(),
                current_session_id: session_id,
            }
        },
        fuzzy_match_history,
    )
}

pub(crate) fn fuzzy_match_history(
    snapshot: HistorySnapshot,
) -> BoxFuture<'static, Result<Vec<QueryResult<CommandSearchItemAction>>, DataSourceRunErrorWrapper>>
{
    if !FeatureFlag::HistorySearchRankingV2.is_enabled() {
        return fuzzy_match_history_legacy(snapshot);
    }

    Box::pin(async move {
        let mut results = Vec::new();
        let now = Local::now();
        let is_blank_query = snapshot.query_text.trim().is_empty();
        let tokens = rank::tokenize_query(&snapshot.query_text);

        // History entries are cheap to match (single short string), so we use a large chunk
        // size to reduce yield overhead while still allowing cancellation of stale queries.
        for chunk in snapshot.commands.chunks(CHUNK_SIZE) {
            for entry in chunk {
                let Some((match_result, match_quality)) =
                    rank::match_history_command(entry.command.as_str(), &tokens)
                else {
                    continue;
                };

                let Some(score) = rank::rank(RankInputs {
                    entry: entry.as_ref(),
                    match_quality,
                    now,
                    current_session_id: snapshot.current_session_id,
                    is_blank_query,
                }) else {
                    continue;
                };

                results.push(
                    HistorySearchItem {
                        entry: entry.clone(),
                        match_result,
                        score,
                    }
                    .into(),
                );
            }
            yield_now().await;
        }

        Ok(results)
    })
}

/// Pre-ranking behavior (flag off): the raw Skim score of the whole query.
fn fuzzy_match_history_legacy(
    snapshot: HistorySnapshot,
) -> BoxFuture<'static, Result<Vec<QueryResult<CommandSearchItemAction>>, DataSourceRunErrorWrapper>>
{
    Box::pin(async move {
        let mut results = Vec::new();

        for chunk in snapshot.commands.chunks(CHUNK_SIZE) {
            for entry in chunk {
                let Some(match_result) = fuzzy_match::match_indices_case_insensitive(
                    entry.command.as_str(),
                    snapshot.query_text.as_str(),
                ) else {
                    continue;
                };
                let score = OrderedFloat(match_result.score as f64);

                results.push(
                    HistorySearchItem {
                        entry: entry.clone(),
                        match_result,
                        score,
                    }
                    .into(),
                );
            }
            yield_now().await;
        }

        Ok(results)
    })
}
