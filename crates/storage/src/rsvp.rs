use sqlx::PgPool;

use crate::types::Rsvp;

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct RsvpResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    pub attending: bool,
    pub party_size: i16,
    pub meal_preference: Option<String>,
    pub accommodation_name: Option<String>,
    pub accommodation_address: Option<String>,
    pub phone: Option<String>,
    pub dietary_requirements: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingRsvpSheetSync {
    pub rsvp_id: uuid::Uuid,
    pub payload: serde_json::Value,
}

#[derive(Debug)]
pub enum SubmitRsvpError {
    Duplicate,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for SubmitRsvpError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

pub struct RsvpStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> RsvpStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn responses(&self) -> Result<Vec<RsvpResponse>, sqlx::Error> {
        sqlx::query_as(
            "SELECT r.id, g.name, g.first_name, g.last_name, g.email, r.attending,
                    r.party_size, r.meal_preference, g.accommodation_name,
                    g.accommodation_address, g.phone, g.dietary_requirements, r.created_at
             FROM rsvps r JOIN guests g ON g.id = r.guest_id
             ORDER BY r.created_at DESC, r.id",
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn submit_individual(
        &self,
        first_name: &str,
        last_name: &str,
        email: &str,
        attending: bool,
        accommodation_name: Option<&str>,
        accommodation_address: Option<&str>,
        phone: Option<&str>,
        dietary_requirements: Option<&str>,
        meal_preference: Option<&str>,
    ) -> Result<(), SubmitRsvpError> {
        let mut transaction = self.pool.begin().await?;
        let email = email.trim().to_lowercase();
        let name = format!("{} {}", first_name.trim(), last_name.trim());
        let guest_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO guests
                (name, first_name, last_name, email, plus_one, accommodation_name,
                 accommodation_address, phone, dietary_requirements)
             VALUES ($1, $2, $3, $4, FALSE, $5, $6, $7, $8)
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(&name)
        .bind(first_name.trim())
        .bind(last_name.trim())
        .bind(&email)
        .bind(accommodation_name)
        .bind(accommodation_address)
        .bind(phone)
        .bind(dietary_requirements)
        .fetch_optional(&mut *transaction)
        .await?;

        let Some(guest_id) = guest_id else {
            transaction.rollback().await?;
            return Err(SubmitRsvpError::Duplicate);
        };

        let party_size = i16::from(attending);
        let rsvp = sqlx::query_as::<_, Rsvp>(
            "INSERT INTO rsvps (guest_id, attending, party_size, meal_preference)
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(guest_id)
        .bind(attending)
        .bind(party_size)
        .bind(meal_preference)
        .fetch_one(&mut *transaction)
        .await?;

        let payload = serde_json::json!({
            "rsvp_id": rsvp.id,
            "submitted_at": rsvp.created_at.to_rfc3339(),
            "first_name": first_name.trim(),
            "last_name": last_name.trim(),
            "guest_name": name,
            "email": email,
            "attendance": if attending { "Attending" } else { "Declining" },
            "accommodation_name": accommodation_name,
            "accommodation_address": accommodation_address,
            "phone": phone,
            "dietary_requirements": dietary_requirements,
            "main_course_choice": meal_preference,
            "legacy_party_size": serde_json::Value::Null
        });
        sqlx::query(
            "INSERT INTO rsvp_sheets_outbox (rsvp_id, payload) VALUES ($1, $2)",
        )
        .bind(rsvp.id)
        .bind(payload)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(())
    }

    pub async fn pending_sheet_syncs(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingRsvpSheetSync>, sqlx::Error> {
        sqlx::query_as(
            "SELECT rsvp_id, payload
             FROM rsvp_sheets_outbox
             WHERE delivered_at IS NULL AND next_attempt_at <= NOW()
             ORDER BY created_at, rsvp_id
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }

    pub async fn mark_sheet_sync_delivered(&self, rsvp_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE rsvp_sheets_outbox
             SET delivered_at = NOW(), last_error = NULL
             WHERE rsvp_id = $1 AND delivered_at IS NULL",
        )
        .bind(rsvp_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_sheet_sync_failed(
        &self,
        rsvp_id: uuid::Uuid,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE rsvp_sheets_outbox
             SET attempts = attempts + 1,
                 last_error = left($2, 1000),
                 next_attempt_at = NOW() + make_interval(secs => LEAST(30 * power(2, attempts), 3600)::int)
             WHERE rsvp_id = $1 AND delivered_at IS NULL",
        )
        .bind(rsvp_id)
        .bind(error)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn submit(
        &self,
        name: &str,
        email: &str,
        attending: bool,
        party_size: i16,
    ) -> Result<(), SubmitRsvpError> {
        let mut transaction = self.pool.begin().await?;
        let email = email.trim().to_lowercase();
        let new_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO guests (name, email, plus_one) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(name)
        .bind(&email)
        .bind(party_size == 2)
        .fetch_optional(&mut *transaction)
        .await?;
        let guest_id = match new_id {
            Some(id) => id,
            None => {
                sqlx::query_scalar(
                    "SELECT id FROM guests WHERE lower(btrim(email)) = lower(btrim($1))",
                )
                .bind(&email)
                .fetch_one(&mut *transaction)
                .await?
            }
        };
        let inserted = sqlx::query(
            "INSERT INTO rsvps (guest_id, attending, party_size) VALUES ($1, $2, $3)
             ON CONFLICT (guest_id) DO NOTHING",
        )
        .bind(guest_id)
        .bind(attending)
        .bind(party_size)
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() == 0 {
            transaction.rollback().await?;
            return Err(SubmitRsvpError::Duplicate);
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_by_guest(&self, guest_id: uuid::Uuid) -> Result<Vec<Rsvp>, sqlx::Error> {
        sqlx::query_as::<_, Rsvp>(
            "SELECT * FROM rsvps WHERE guest_id = $1 ORDER BY created_at DESC",
        )
        .bind(guest_id)
        .fetch_all(self.pool)
        .await
    }

    pub async fn get(&self, id: uuid::Uuid) -> Result<Option<Rsvp>, sqlx::Error> {
        sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    pub async fn create(&self, rsvp: &Rsvp) -> Result<Rsvp, sqlx::Error> {
        sqlx::query_as::<_, Rsvp>(
            r#"
            INSERT INTO rsvps (id, guest_id, attending, meal_preference, party_size, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(rsvp.id)
        .bind(rsvp.guest_id)
        .bind(rsvp.attending)
        .bind(&rsvp.meal_preference)
        .bind(rsvp.party_size)
        .fetch_one(self.pool)
        .await
    }

    pub async fn update(&self, rsvp: &Rsvp) -> Result<Rsvp, sqlx::Error> {
        sqlx::query_as::<_, Rsvp>(
            r#"
            UPDATE rsvps
            SET attending = $2, meal_preference = $3, party_size = $4, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(rsvp.id)
        .bind(rsvp.attending)
        .bind(&rsvp.meal_preference)
        .bind(rsvp.party_size)
        .fetch_one(self.pool)
        .await
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM rsvps WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
