//! Phase 07 background pipeline.
//!
//! A single reusable pass — ingest (Phase 03) → extract (Phase 04) → regenerate
//! notifications (Phase 06) — shared by the manual admin triggers and the
//! scheduled `sync-once` cron job. A Postgres advisory lock ensures two passes
//! (e.g. a cron run and a manual trigger) can never overlap, and every pass is
//! recorded in `sync_runs` for the dashboard's system-status widget.

use serde::Serialize;

use storage::types::SyncRunCounts;

use crate::{Context, extract, ingest};

/// Stable, arbitrary key for the pipeline advisory lock (`pg_try_advisory_lock`).
const PIPELINE_LOCK_KEY: i64 = 0x0077_6564_6469_6e67; // "wedding"

/// Cap on stored error text so a burst of provider errors can't bloat a row.
const MAX_ERROR_LEN: usize = 1000;

/// Outcome of a pipeline pass, returned as JSON to manual callers and printed by
/// the `sync-once` subcommand.
#[derive(Debug, Serialize)]
pub struct PipelineReport {
    /// True when another pass held the lock and this one did no work.
    pub skipped: bool,
    pub status: String,
    pub sources_new: i32,
    pub sources_updated: i32,
    pub todos_created: i32,
    pub events_created: i32,
    pub deadlines_created: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PipelineReport {
    fn skipped() -> Self {
        Self {
            skipped: true,
            status: "skipped".to_string(),
            sources_new: 0,
            sources_updated: 0,
            todos_created: 0,
            events_created: 0,
            deadlines_created: 0,
            error: None,
        }
    }
}

/// Runs one pipeline pass under an advisory lock, recording a `sync_runs` row.
/// Returns a `skipped` report if another pass is already running.
pub async fn run_once(ctx: &Context, trigger: &str) -> Result<PipelineReport, sqlx::Error> {
    let pool = ctx.pool();
    // Hold this connection for the whole pass so the session-scoped advisory
    // lock stays held; the pipeline's own queries use other pool connections.
    let mut lock_conn = pool.acquire().await?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(PIPELINE_LOCK_KEY)
        .fetch_one(&mut *lock_conn)
        .await?;
    if !acquired {
        return Ok(PipelineReport::skipped());
    }

    let result = run_inner(ctx, trigger).await;

    // Always release the lock, even if the pass errored.
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(PIPELINE_LOCK_KEY)
        .execute(&mut *lock_conn)
        .await;

    result
}

async fn run_inner(ctx: &Context, trigger: &str) -> Result<PipelineReport, sqlx::Error> {
    let storage = ctx.storage();
    let run_id = storage.sync_runs().start(trigger).await?;

    let mut counts = SyncRunCounts::default();
    let mut errors: Vec<String> = Vec::new();

    // 1. Ingest Gmail + Drive for every linked account (per-account errors are
    //    isolated and surfaced, not fatal).
    if let Some(google) = ctx.google() {
        match ingest::sync_all(&storage, google).await {
            Ok(report) => {
                for acct in &report.accounts {
                    counts.sources_new += (acct.gmail.new + acct.drive.new) as i32;
                    counts.sources_updated += (acct.gmail.updated + acct.drive.updated) as i32;
                    if let Some(e) = &acct.gmail.error {
                        errors.push(format!("{} gmail: {e}", acct.account_email));
                    }
                    if let Some(e) = &acct.drive.error {
                        errors.push(format!("{} drive: {e}", acct.account_email));
                    }
                }
            }
            Err(e) => errors.push(format!("ingest: {e}")),
        }
    }

    // 2. Extract structured items from new sources. `extract_pending` also
    //    regenerates notifications at the end of its pass.
    if let Some(extractor) = ctx.llm() {
        match extract::extract_pending(
            &storage,
            extractor,
            extractor.model(),
            ctx.llm_max_batch(),
        )
        .await
        {
            Ok(rep) => {
                counts.todos_created += rep.todos_created as i32;
                counts.events_created += rep.events_created as i32;
                counts.deadlines_created += rep.deadlines_created as i32;
                if rep.failed > 0 {
                    errors.push(format!("extraction: {} source(s) failed", rep.failed));
                }
            }
            Err(e) => errors.push(format!("extract: {e}")),
        }
    } else if let Err(e) = storage.notifications().generate().await {
        // No LLM configured — still refresh reminders off existing items.
        errors.push(format!("notifications: {e}"));
    }

    let status = if errors.is_empty() { "success" } else { "error" };
    let error_text = if errors.is_empty() {
        None
    } else {
        Some(truncate(errors.join("; ")))
    };
    storage
        .sync_runs()
        .finish(run_id, status, &counts, error_text.as_deref())
        .await?;

    Ok(PipelineReport {
        skipped: false,
        status: status.to_string(),
        sources_new: counts.sources_new,
        sources_updated: counts.sources_updated,
        todos_created: counts.todos_created,
        events_created: counts.events_created,
        deadlines_created: counts.deadlines_created,
        error: error_text,
    })
}

fn truncate(mut s: String) -> String {
    if s.chars().count() > MAX_ERROR_LEN {
        s = s.chars().take(MAX_ERROR_LEN).collect();
        s.push('…');
    }
    s
}
