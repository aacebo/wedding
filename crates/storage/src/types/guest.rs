#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Guest {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub plus_one: bool,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub accommodation_name: Option<String>,
    pub accommodation_address: Option<String>,
    pub phone: Option<String>,
    pub dietary_requirements: Option<String>,
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
            first_name: None,
            last_name: None,
            accommodation_name: None,
            accommodation_address: None,
            phone: None,
            dietary_requirements: None,
            created_at: now,
            updated_at: now,
        }
    }
}
