#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Guest {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub plus_one: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Guest {
    pub fn new(name: impl Into<String>, email: impl Into<String>, plus_one: bool) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            email: email.into(),
            plus_one,
            created_at: now,
            updated_at: now,
        }
    }
}
