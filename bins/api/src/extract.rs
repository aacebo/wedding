//! Phase 04 extraction service.
//!
//! Walks the unprocessed `comm_sources` worklist, asks the LLM to distill each
//! into structured todos/timeline events/deadlines, and persists the result.
//!
//! Idempotency: before inserting fresh output for a source we delete only its
//! AI-created, still-open items (see `AdminItemStorage::clear_ai_open_for_source`),
//! so human edits and completed/dismissed items survive re-runs. A source is
//! stamped `processed_at` only on success; on LLM failure we record an error
//! `llm_runs` row and leave the source unprocessed so the next run retries it.

use chrono::NaiveDate;

use llm::{LlmExtractor, SourceInput, estimate_cost};
use storage::Storage;
use storage::types::{NewAdminTodo, NewDeadline, NewLlmRun, NewTimelineEvent};

/// Summary of one extraction pass, returned to the caller as JSON.
#[derive(Debug, Default, serde::Serialize)]
pub struct ExtractReport {
    /// Sources successfully processed this run.
    pub processed: u64,
    /// Sources whose extraction call failed (left unprocessed for retry).
    pub failed: u64,
    pub todos_created: u64,
    pub events_created: u64,
    pub deadlines_created: u64,
    /// Estimated USD cost of the OpenAI calls made this run.
    pub cost_estimate: f64,
    /// Running totals across all extraction output (not just this run).
    pub total_todos: i64,
    pub total_events: i64,
    pub total_deadlines: i64,
}

/// Runs extraction over up to `max_batch` unprocessed sources.
pub async fn extract_pending<E: LlmExtractor>(
    storage: &Storage<'_>,
    extractor: &E,
    model: &str,
    max_batch: i64,
) -> Result<ExtractReport, sqlx::Error> {
    let sources = storage.comm_sources().unprocessed(max_batch).await?;
    let mut report = ExtractReport::default();

    for source in sources {
        let input = SourceInput {
            kind: source.kind.clone(),
            sender: source.sender.clone(),
            title: source.title.clone(),
            occurred_at: source.occurred_at.map(|d| d.date_naive().to_string()),
            body: source.body_text.clone(),
        };

        match extractor.extract(&input).await {
            Ok(extraction) => {
                // Replace prior AI output for this source, preserving human edits.
                storage
                    .admin_items()
                    .clear_ai_open_for_source(source.id)
                    .await?;

                for t in &extraction.todos {
                    if t.title.trim().is_empty() {
                        continue;
                    }
                    storage
                        .admin_items()
                        .insert_todo(
                            &NewAdminTodo {
                                source_id: Some(source.id),
                                title: t.title.clone(),
                                owner: t.owner.clone(),
                                due_date: parse_date(t.due_date.as_deref()),
                                priority: normalize_level(&t.priority),
                                confidence: t.confidence as f32,
                                notes: t.notes.clone(),
                            },
                            "ai",
                        )
                        .await?;
                    report.todos_created += 1;
                }

                for e in &extraction.timeline_events {
                    if e.title.trim().is_empty() {
                        continue;
                    }
                    storage
                        .admin_items()
                        .insert_event(
                            &NewTimelineEvent {
                                source_id: Some(source.id),
                                title: e.title.clone(),
                                event_date: parse_date(e.date.as_deref()),
                                category: e.category.clone(),
                                confidence: e.confidence as f32,
                            },
                            "ai",
                        )
                        .await?;
                    report.events_created += 1;
                }

                for d in &extraction.deadlines {
                    if d.title.trim().is_empty() {
                        continue;
                    }
                    storage
                        .admin_items()
                        .insert_deadline(
                            &NewDeadline {
                                source_id: Some(source.id),
                                title: d.title.clone(),
                                due_date: parse_date(d.due_date.as_deref()),
                                severity: normalize_level(&d.severity),
                                confidence: d.confidence as f32,
                            },
                            "ai",
                        )
                        .await?;
                    report.deadlines_created += 1;
                }

                let cost = estimate_cost(
                    &extraction.usage.model,
                    extraction.usage.prompt_tokens,
                    extraction.usage.completion_tokens,
                );
                report.cost_estimate += cost;

                storage
                    .admin_items()
                    .insert_llm_run(&NewLlmRun {
                        source_id: Some(source.id),
                        model: extraction.usage.model.clone(),
                        prompt_tokens: extraction.usage.prompt_tokens,
                        completion_tokens: extraction.usage.completion_tokens,
                        cost_estimate: cost as f32,
                        status: "success".to_string(),
                        error: None,
                    })
                    .await?;

                storage.comm_sources().mark_processed(source.id).await?;
                report.processed += 1;
            }
            Err(err) => {
                // Record the failure but leave the source unprocessed for retry.
                storage
                    .admin_items()
                    .insert_llm_run(&NewLlmRun {
                        source_id: Some(source.id),
                        model: model.to_string(),
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        cost_estimate: 0.0,
                        status: "error".to_string(),
                        error: Some(err.to_string()),
                    })
                    .await?;
                report.failed += 1;
            }
        }
    }

    let (todos, events, deadlines) = storage.admin_items().counts().await?;
    report.total_todos = todos;
    report.total_events = events;
    report.total_deadlines = deadlines;

    Ok(report)
}

/// Parses an ISO `YYYY-MM-DD` string into a `NaiveDate`, ignoring anything the
/// model returns in an unexpected shape.
fn parse_date(s: Option<&str>) -> Option<NaiveDate> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Clamps a model-supplied priority/severity to the allowed set, defaulting to
/// `med` for anything unrecognized.
fn normalize_level(level: &str) -> String {
    match level.trim().to_lowercase().as_str() {
        "low" => "low".to_string(),
        "high" => "high".to_string(),
        _ => "med".to_string(),
    }
}
