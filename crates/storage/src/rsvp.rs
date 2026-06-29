use sqlx::PgPool;

use crate::types::Rsvp;

pub struct RsvpStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> RsvpStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_guest(&self, guest_id: uuid::Uuid) -> Result<Vec<Rsvp>, sqlx::Error> {
        sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE guest_id = $1 ORDER BY created_at DESC")
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
            INSERT INTO rsvps (id, guest_id, attending, meal_preference, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(rsvp.id)
        .bind(rsvp.guest_id)
        .bind(rsvp.attending)
        .bind(&rsvp.meal_preference)
        .fetch_one(self.pool)
        .await
    }

    pub async fn update(&self, rsvp: &Rsvp) -> Result<Rsvp, sqlx::Error> {
        sqlx::query_as::<_, Rsvp>(
            r#"
            UPDATE rsvps
            SET attending = $2, meal_preference = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(rsvp.id)
        .bind(rsvp.attending)
        .bind(&rsvp.meal_preference)
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
