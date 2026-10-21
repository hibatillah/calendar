use calendar_backend::{repository::Repository, router, schema};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_address = std::env::var("CALENDAR_BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:47821".into())
        .parse::<std::net::SocketAddr>()?;
    if !bind_address.ip().is_loopback()
        && std::env::var("CALENDAR_PRIVATE_SERVER").as_deref() != Ok("true")
    {
        return Err("Non-loopback binding requires CALENDAR_PRIVATE_SERVER=true. Public multi-user hosting is not implemented.".into());
    }
    let _logging = calendar_telemetry::init("backend");
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
    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    tracing::info!(
        action = "backend_ready",
        address = %bind_address,
        mode = "google_calendar",
        "Backend ready"
    );
    axum::serve(listener, router(repository, token))
        .with_graceful_shutdown(async {
            shutdown_signal().await;
        })
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
