use calendar_core::{CalendarEvent, EventCommand};
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(&'static str),
    #[error("This calendar is read-only")]
    ReadOnly,
    #[error("{0}")]
    Storage(&'static str),
}

#[derive(Clone)]
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    pub async fn open(url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::raw_sql("CREATE TABLE IF NOT EXISTS events (event_id TEXT PRIMARY KEY, payload TEXT NOT NULL, revision INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS operations (operation_id TEXT PRIMARY KEY, request TEXT NOT NULL, result TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);").execute(&pool).await?;
        Ok(Self { pool })
    }
    pub async fn remove_preview_data(&self) -> Result<(), sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("DELETE FROM events WHERE json_extract(payload, '$.calendar_id') IN ('work', 'personal')").execute(&mut *transaction).await?;
        sqlx::query("DELETE FROM operations WHERE json_extract(request, '$.event.calendar_id') IN ('work', 'personal')").execute(&mut *transaction).await?;
        sqlx::query("DELETE FROM metadata WHERE key='seeded'")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await
    }
    #[tracing::instrument(skip_all, name = "database.events")]
    pub async fn events(&self) -> Result<Vec<CalendarEvent>, RepositoryError> {
        let rows: Vec<String> = sqlx::query_scalar("SELECT payload FROM events")
            .fetch_all(&self.pool)
            .await
            .map_err(|_| RepositoryError::Storage("Cannot read local calendar"))?;
        rows.into_iter()
            .map(|payload| {
                serde_json::from_str(&payload)
                    .map_err(|_| RepositoryError::Storage("Invalid stored event"))
            })
            .collect()
    }
    #[tracing::instrument(skip_all, name = "database.save")]
    pub async fn save(&self, command: EventCommand) -> Result<CalendarEvent, RepositoryError> {
        command.event.validate().map_err(RepositoryError::Invalid)?;
        if command.operation_id.is_empty() || command.event.event_id.is_empty() {
            return Err(RepositoryError::Invalid(
                "Missing operation or event ID".into(),
            ));
        }
        let serialized_request = serde_json::to_string(&command).unwrap();
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| RepositoryError::Storage("Database unavailable"))?;
        if let Some(row) =
            sqlx::query("SELECT request, result FROM operations WHERE operation_id=?")
                .bind(&command.operation_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|_| RepositoryError::Storage("Cannot read operation"))?
        {
            if row.get::<String, _>("request") != serialized_request {
                return Err(RepositoryError::Conflict(
                    "Operation ID reused with different data",
                ));
            }
            return serde_json::from_str(&row.get::<String, _>("result"))
                .map_err(|_| RepositoryError::Storage("Invalid operation result"));
        }
        let existing: Option<(i64, String)> =
            sqlx::query_as("SELECT revision, payload FROM events WHERE event_id=?")
                .bind(&command.event.event_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|_| RepositoryError::Storage("Cannot read event"))?;
        match &existing {
            Some((revision, payload)) => {
                let stored: CalendarEvent = serde_json::from_str(payload)
                    .map_err(|_| RepositoryError::Storage("Invalid stored event"))?;
                if stored.read_only {
                    return Err(RepositoryError::ReadOnly);
                }
                if *revision != command.expected_revision {
                    return Err(RepositoryError::Conflict(
                        "Event changed elsewhere. Refresh before saving.",
                    ));
                }
            }
            None if command.expected_revision != -1 => {
                return Err(RepositoryError::Conflict(
                    "Event no longer exists. Refresh before saving.",
                ));
            }
            None => (),
        }
        let mut event = command.event;
        event.revision = existing.map(|(revision, _)| revision + 1).unwrap_or(0);
        let payload = serde_json::to_string(&event).unwrap();
        sqlx::query("INSERT INTO events VALUES (?, ?, ?) ON CONFLICT(event_id) DO UPDATE SET payload=excluded.payload, revision=excluded.revision")
            .bind(&event.event_id).bind(&payload).bind(event.revision).execute(&mut *transaction).await.map_err(|_| RepositoryError::Storage("Cannot save event"))?;
        sqlx::query("INSERT INTO operations VALUES (?, ?, ?)")
            .bind(&command.operation_id)
            .bind(serialized_request)
            .bind(&payload)
            .execute(&mut *transaction)
            .await
            .map_err(|_| RepositoryError::Storage("Cannot record operation"))?;
        transaction
            .commit()
            .await
            .map_err(|_| RepositoryError::Storage("Cannot commit event"))?;
        tracing::info!(action = "event_saved", event_id = %event.event_id, operation_id = %command.operation_id, revision = event.revision, "Local mutation committed");
        Ok(event)
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[tokio::test]
    async fn preview_cleanup_preserves_provider_rows() {
        let repository = Repository::open("sqlite::memory:").await.unwrap();
        for (identifier, calendar) in [("sample", "work"), ("provider", "google-calendar")] {
            let payload = serde_json::json!({"calendar_id":calendar}).to_string();
            sqlx::query("INSERT INTO events VALUES (?,?,0)")
                .bind(identifier)
                .bind(payload)
                .execute(&repository.pool)
                .await
                .unwrap();
        }
        repository.remove_preview_data().await.unwrap();
        repository.remove_preview_data().await.unwrap();
        let identifiers: Vec<String> = sqlx::query_scalar("SELECT event_id FROM events")
            .fetch_all(&repository.pool)
            .await
            .unwrap();
        assert_eq!(identifiers, vec!["provider"]);
    }
}
