use randevu_backend::{repository::Repository, router, schema};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logging = randevu_telemetry::init("backend");
    std::fs::create_dir_all(".local")?;
    let repository = Repository::open("sqlite:.local/backend.db?mode=rwc").await?;
    repository.remove_preview_data().await?;
    std::fs::create_dir_all("contracts/graphql")?;
    std::fs::write(
        "contracts/graphql/schema.graphql",
        schema(repository.clone()).sdl(),
    )?;
    let token = match std::fs::read_to_string(".local/session-token") {
        Ok(token) => token,
        Err(_) => {
            let token = uuid::Uuid::new_v4().to_string();
            std::fs::write(".local/session-token", &token)?;
            token
        }
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:47821").await?;
    tracing::info!(
        action = "backend_ready",
        address = "127.0.0.1:47821",
        mode = "google_calendar",
        "Backend ready"
    );
    axum::serve(listener, router(repository, token))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
