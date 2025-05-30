use color_eyre::Result;
use sqlx::SqliteExecutor;

pub struct Player {
    // this is disgusting - converting to a string is gross but unfortunately
    // sqlite doesn't natively have a u64 type :(
    pub id: String,
    pub infected: bool,
    pub total_messages: i64,
    pub sanitized_messages: i64,
    pub last_action: i64,
}

#[derive(sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum InfectionEvent {
    Infected,
    Cured,
}

impl From<String> for InfectionEvent {
    fn from(value: String) -> Self {
        match value.as_str() {
            "cured" => Self::Cured,
            _ => Self::Infected,
        }
    }
}

pub struct InfectionRecord {
    pub event: InfectionEvent,
    pub target: String,
    pub source: Option<String>,
    pub reason: Option<String>,
    pub recorded_at: i64,
    pub target_total_messages: i64,
    pub target_sanitized_messages: i64,
}

impl InfectionRecord {
    pub async fn save(self, e: impl SqliteExecutor<'_>) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO infection_records
            (event, target, source, reason, recorded_at, target_total_messages, target_sanitized_messages) 
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            self.event,
            self.target,
            self.source,
            self.reason,
            self.recorded_at,
            self.target_total_messages,
            self.target_sanitized_messages,
        ).execute(e).await?;
        Ok(())
    }
}
