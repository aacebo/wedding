#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Rsvp {
    pub id: uuid::Uuid,
    pub guest_id: uuid::Uuid,
    pub attending: bool,
    pub meal_preference: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Rsvp {
    pub fn new(guest_id: uuid::Uuid, attending: bool, meal_preference: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            guest_id,
            attending,
            meal_preference,
            created_at: now,
            updated_at: now,
        }
    }
}
