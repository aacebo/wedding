use sqlx::PgPool;

use crate::types::Guest;

pub struct GuestStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> GuestStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Guest>, sqlx::Error> {
        sqlx::query_as::<_, Guest>("SELECT * FROM guests ORDER BY created_at DESC")
            .fetch_all(self.pool)
            .await
    }

    pub async fn get(&self, id: uuid::Uuid) -> Result<Option<Guest>, sqlx::Error> {
        sqlx::query_as::<_, Guest>("SELECT * FROM guests WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    pub async fn create(&self, guest: &Guest) -> Result<Guest, sqlx::Error> {
        sqlx::query_as::<_, Guest>(
            r#"
            INSERT INTO guests (id, name, email, plus_one, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(guest.id)
        .bind(&guest.name)
        .bind(&guest.email)
        .bind(guest.plus_one)
        .fetch_one(self.pool)
        .await
    }

    pub async fn update(&self, guest: &Guest) -> Result<Guest, sqlx::Error> {
        sqlx::query_as::<_, Guest>(
            r#"
            UPDATE guests
            SET name = $2, email = $3, plus_one = $4, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(guest.id)
        .bind(&guest.name)
        .bind(&guest.email)
        .bind(guest.plus_one)
        .fetch_one(self.pool)
        .await
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM guests WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
