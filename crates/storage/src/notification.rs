use sqlx::PgPool;

use crate::types::Notification;

/// Storage + generation for in-page deadline/todo reminders.
///
/// [`generate`](Self::generate) is idempotent: it upserts one notification per
/// referenced item (keyed on `ref_type` + `ref_id`), escalating the `kind` in
/// place as a deadline moves from soon to overdue, and prunes reminders whose
/// underlying item is no longer open/dated. Existing read/dismissed state is
/// preserved across runs.
pub struct NotificationStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> NotificationStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Number of days ahead a deadline/todo is considered "soon".
    const SOON_WINDOW_DAYS: i32 = 14;

    /// (Re)computes reminders from `deadlines` and `todos_admin`, returning the
    /// number of reminders that currently exist (unread + read, excluding
    /// dismissed).
    pub async fn generate(&self) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let window = Self::SOON_WINDOW_DAYS;

        // Prune reminders whose source item is gone, closed, or undated so we
        // don't keep nagging about resolved work.
        sqlx::query(
            r#"
            DELETE FROM notifications n
             WHERE n.ref_type = 'deadline'
               AND NOT EXISTS (
                   SELECT 1 FROM deadlines d
                    WHERE d.id = n.ref_id
                      AND d.status = 'open'
                      AND d.due_date IS NOT NULL
               )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM notifications n
             WHERE n.ref_type = 'todo'
               AND NOT EXISTS (
                   SELECT 1 FROM todos_admin t
                    WHERE t.id = n.ref_id
                      AND t.status = 'open'
                      AND t.due_date IS NOT NULL
               )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // Deadlines: overdue (past due) or soon (within the window).
        sqlx::query(
            r#"
            INSERT INTO notifications (kind, ref_type, ref_id, title, body, due_date)
            SELECT
                CASE WHEN d.due_date < CURRENT_DATE THEN 'deadline_overdue'
                     ELSE 'deadline_soon' END,
                'deadline', d.id, d.title,
                CASE WHEN d.due_date < CURRENT_DATE
                     THEN 'Overdue since ' || to_char(d.due_date, 'Mon DD')
                     WHEN d.due_date = CURRENT_DATE THEN 'Due today'
                     ELSE 'Due ' || to_char(d.due_date, 'Mon DD')
                          || ' (' || (d.due_date - CURRENT_DATE) || ' days)' END,
                d.due_date
              FROM deadlines d
             WHERE d.status = 'open'
               AND d.due_date IS NOT NULL
               AND d.due_date <= CURRENT_DATE + ($1 || ' days')::interval
            ON CONFLICT (ref_type, ref_id) DO UPDATE
               SET kind = EXCLUDED.kind,
                   title = EXCLUDED.title,
                   body = EXCLUDED.body,
                   due_date = EXCLUDED.due_date,
                   updated_at = NOW()
            "#,
        )
        .bind(window)
        .execute(&mut *tx)
        .await?;

        // Todos with a due date within the window (or overdue).
        sqlx::query(
            r#"
            INSERT INTO notifications (kind, ref_type, ref_id, title, body, due_date)
            SELECT
                'todo_due', 'todo', t.id, t.title,
                CASE WHEN t.due_date < CURRENT_DATE
                     THEN 'Overdue since ' || to_char(t.due_date, 'Mon DD')
                     WHEN t.due_date = CURRENT_DATE THEN 'Due today'
                     ELSE 'Due ' || to_char(t.due_date, 'Mon DD')
                          || ' (' || (t.due_date - CURRENT_DATE) || ' days)' END,
                t.due_date
              FROM todos_admin t
             WHERE t.status = 'open'
               AND t.due_date IS NOT NULL
               AND t.due_date <= CURRENT_DATE + ($1 || ' days')::interval
            ON CONFLICT (ref_type, ref_id) DO UPDATE
               SET kind = EXCLUDED.kind,
                   title = EXCLUDED.title,
                   body = EXCLUDED.body,
                   due_date = EXCLUDED.due_date,
                   updated_at = NOW()
            "#,
        )
        .bind(window)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.active_count().await
    }

    /// Non-dismissed reminders, overdue first then soonest due date.
    pub async fn list(&self) -> Result<Vec<Notification>, sqlx::Error> {
        sqlx::query_as::<_, Notification>(
            r#"
            SELECT id, kind, ref_type, ref_id, title, body, due_date, status,
                   created_at, updated_at
              FROM notifications
             WHERE status <> 'dismissed'
             ORDER BY
               CASE WHEN kind = 'deadline_overdue' THEN 0 ELSE 1 END,
               due_date ASC NULLS LAST,
               created_at DESC
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    /// Count of unread reminders — drives the nav badge.
    pub async fn unread_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE status = 'unread'")
            .fetch_one(self.pool)
            .await
    }

    /// Count of reminders still visible in the panel (unread or read).
    pub async fn active_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE status <> 'dismissed'")
            .fetch_one(self.pool)
            .await
    }

    pub async fn set_status(&self, id: uuid::Uuid, status: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE notifications SET status = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Marks every unread reminder as read (used by "mark all read").
    pub async fn mark_all_read(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE notifications SET status = 'read', updated_at = NOW() WHERE status = 'unread'",
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
