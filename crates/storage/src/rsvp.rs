use sqlx::PgPool;

use crate::types::Rsvp;

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct RsvpResponse {
    pub name: String,
    pub email: String,
    pub attending: bool,
    pub party_size: i16,
    pub created_at: chrono::DateTime<chrono::Utc>,
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
            "SELECT g.name, g.email, r.attending, r.party_size, r.created_at
             FROM rsvps r JOIN guests g ON g.id = r.guest_id
             ORDER BY r.created_at DESC, r.id",
        )
        .fetch_all(self.pool)
        .await
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
